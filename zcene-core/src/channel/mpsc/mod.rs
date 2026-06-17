// Taken from the futures-channel crate (https://github.com/rust-lang/futures-rs/tree/master/futures-channel)

use alloc::sync::Arc;
use core::alloc::Allocator;
use core::fmt;
use core::future::Future;
use core::hint;
use core::pin::Pin;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::SeqCst;
use futures_core::FusedFuture;
use futures_core::stream::{FusedStream, Stream};
use futures_core::task::__internal::AtomicWaker;
use futures_core::task::{Context, Poll, Waker};
use spin::Mutex;

use self::queue::Queue;

mod queue;

////////////////////////////////////////////////////////////////////////////////////////////////////

struct UnboundedSenderInner<T, A: Allocator + Clone> {
    inner: Arc<UnboundedInner<T, A>, A>,
}

struct BoundedSenderInner<T, A: Allocator + Clone> {
    inner: Arc<BoundedInner<T, A>, A>,
    sender_task: Arc<Mutex<SenderTask>, A>,
    maybe_parked: bool,
}

impl<T, A: Allocator + Clone> Unpin for UnboundedSenderInner<T, A> {}
impl<T, A: Allocator + Clone> Unpin for BoundedSenderInner<T, A> {}

pub struct Sender<T, A: Allocator + Clone>(Option<BoundedSenderInner<T, A>>);
pub struct UnboundedSender<T, A: Allocator + Clone>(Option<UnboundedSenderInner<T, A>>);

pub struct Receiver<T, A: Allocator + Clone> {
    inner: Option<Arc<BoundedInner<T, A>, A>>,
}

pub struct UnboundedReceiver<T, A: Allocator + Clone> {
    inner: Option<Arc<UnboundedInner<T, A>, A>>,
}

impl<T, A: Allocator + Clone> Unpin for UnboundedReceiver<T, A> {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendError {
    kind: SendErrorKind,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TrySendError<T> {
    err: SendError,
    val: T,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SendErrorKind {
    Full,
    Disconnected,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryRecvError {
    Empty,
    Closed,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct RecvError;

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Recv<'a, St: ?Sized> {
    stream: &'a mut St,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_full() {
            write!(f, "send failed because channel is full")
        } else {
            write!(f, "send failed because receiver is gone")
        }
    }
}

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "receive failed because channel is empty and closed")
    }
}

impl SendError {
    pub fn is_full(&self) -> bool {
        match self.kind {
            SendErrorKind::Full => true,
            _ => false,
        }
    }

    pub fn is_disconnected(&self) -> bool {
        match self.kind {
            SendErrorKind::Disconnected => true,
            _ => false,
        }
    }
}

impl TryRecvError {
    pub fn is_empty(&self) -> bool {
        matches!(self, TryRecvError::Empty)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, TryRecvError::Closed)
    }
}

impl<T> fmt::Debug for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrySendError").field("kind", &self.err.kind).finish()
    }
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_full() {
            write!(f, "send failed because channel is full")
        } else {
            write!(f, "send failed because receiver is gone")
        }
    }
}

impl<T> TrySendError<T> {
    pub fn is_full(&self) -> bool {
        self.err.is_full()
    }

    pub fn is_disconnected(&self) -> bool {
        self.err.is_disconnected()
    }

    pub fn into_inner(self) -> T {
        self.val
    }

    pub fn into_send_error(self) -> SendError {
        self.err
    }
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryRecvError::Empty => write!(f, "receive failed because channel is empty"),
            TryRecvError::Closed => write!(f, "receive failed because channel is closed"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

struct UnboundedInner<T, A: Allocator + Clone> {
    state: AtomicUsize,
    message_queue: Queue<T, A>,
    num_senders: AtomicUsize,
    recv_task: AtomicWaker,
}

struct BoundedInner<T, A: Allocator + Clone> {
    buffer: usize,
    state: AtomicUsize,
    message_queue: Queue<T, A>,
    parked_queue: Queue<Arc<Mutex<SenderTask>, A>, A>,
    num_senders: AtomicUsize,
    recv_task: AtomicWaker,
}

#[derive(Clone, Copy)]
struct State {
    is_open: bool,
    num_messages: usize,
}

const OPEN_MASK: usize = usize::MAX - (usize::MAX >> 1);
const INIT_STATE: usize = OPEN_MASK;
const MAX_CAPACITY: usize = !(OPEN_MASK);
const MAX_BUFFER: usize = MAX_CAPACITY >> 1;

struct SenderTask {
    task: Option<Waker>,
    is_parked: bool,
}

impl SenderTask {
    fn new() -> Self {
        Self { task: None, is_parked: false }
    }

    fn notify(&mut self) {
        self.is_parked = false;
        if let Some(task) = self.task.take() {
            task.wake();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn channel_in<T, A: Allocator + Clone>(
    buffer: usize,
    alloc: A,
) -> (Sender<T, A>, Receiver<T, A>) {
    assert!(buffer < MAX_BUFFER, "requested buffer size too large");

    let inner = Arc::new_in(
        BoundedInner {
            buffer,
            state: AtomicUsize::new(INIT_STATE),
            message_queue: Queue::new_in(alloc.clone()),
            parked_queue: Queue::new_in(alloc.clone()),
            num_senders: AtomicUsize::new(1),
            recv_task: AtomicWaker::new(),
        },
        alloc.clone(),
    );

    let tx = BoundedSenderInner {
        inner: inner.clone(),
        sender_task: Arc::new_in(Mutex::new(SenderTask::new()), alloc),
        maybe_parked: false,
    };

    let rx = Receiver { inner: Some(inner) };

    (Sender(Some(tx)), rx)
}

pub fn unbounded_in<T, A: Allocator + Clone>(
    alloc: A,
) -> (UnboundedSender<T, A>, UnboundedReceiver<T, A>) {
    let inner = Arc::new_in(
        UnboundedInner {
            state: AtomicUsize::new(INIT_STATE),
            message_queue: Queue::new_in(alloc.clone()),
            num_senders: AtomicUsize::new(1),
            recv_task: AtomicWaker::new(),
        },
        alloc,
    );

    let tx = UnboundedSenderInner { inner: inner.clone() };
    let rx = UnboundedReceiver { inner: Some(inner) };

    (UnboundedSender(Some(tx)), rx)
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> UnboundedSenderInner<T, A> {
    fn poll_ready_nb(&self) -> Poll<Result<(), SendError>> {
        let state = decode_state(self.inner.state.load(SeqCst));
        if state.is_open {
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(SendError { kind: SendErrorKind::Disconnected }))
        }
    }

    fn queue_push_and_signal(&self, msg: T) {
        self.inner.message_queue.push(msg);
        self.inner.recv_task.wake();
    }

    fn inc_num_messages(&self) -> Option<usize> {
        let mut curr = self.inner.state.load(SeqCst);

        loop {
            let mut state = decode_state(curr);

            if !state.is_open {
                return None;
            }

            assert!(
                state.num_messages < MAX_CAPACITY,
                "buffer space exhausted; sending this messages would overflow the state"
            );

            state.num_messages += 1;

            let next = encode_state(&state);
            match self.inner.state.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => return Some(state.num_messages),
                Err(actual) => curr = actual,
            }
        }
    }

    fn same_receiver(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    fn is_connected_to(&self, inner: &Arc<UnboundedInner<T, A>, A>) -> bool {
        Arc::ptr_eq(&self.inner, inner)
    }

    fn ptr(&self) -> *const UnboundedInner<T, A> {
        &*self.inner
    }

    fn is_closed(&self) -> bool {
        !decode_state(self.inner.state.load(SeqCst)).is_open
    }

    fn close_channel(&self) {
        self.inner.set_closed();
        self.inner.recv_task.wake();
    }
}

impl<T, A: Allocator + Clone> BoundedSenderInner<T, A> {
    fn try_send(&mut self, msg: T) -> Result<(), TrySendError<T>> {
        if !self.poll_unparked(None).is_ready() {
            return Err(TrySendError { err: SendError { kind: SendErrorKind::Full }, val: msg });
        }
        self.do_send_b(msg)
    }

    fn do_send_b(&mut self, msg: T) -> Result<(), TrySendError<T>> {
        debug_assert!(self.poll_unparked(None).is_ready());

        let park_self = match self.inc_num_messages() {
            Some(num_messages) => num_messages > self.inner.buffer,
            None => {
                return Err(TrySendError {
                    err: SendError { kind: SendErrorKind::Disconnected },
                    val: msg,
                })
            }
        };

        if park_self {
            self.park();
        }

        self.queue_push_and_signal(msg);
        Ok(())
    }

    fn queue_push_and_signal(&self, msg: T) {
        self.inner.message_queue.push(msg);
        self.inner.recv_task.wake();
    }

    fn inc_num_messages(&self) -> Option<usize> {
        let mut curr = self.inner.state.load(SeqCst);

        loop {
            let mut state = decode_state(curr);

            if !state.is_open {
                return None;
            }

            assert!(
                state.num_messages < MAX_CAPACITY,
                "buffer space exhausted; sending this messages would overflow the state"
            );

            state.num_messages += 1;

            let next = encode_state(&state);
            match self.inner.state.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => return Some(state.num_messages),
                Err(actual) => curr = actual,
            }
        }
    }

    fn park(&mut self) {
        {
            let mut sender = self.sender_task.lock();
            sender.task = None;
            sender.is_parked = true;
        }

        let t = self.sender_task.clone();
        self.inner.parked_queue.push(t);

        let state = decode_state(self.inner.state.load(SeqCst));
        self.maybe_parked = state.is_open;
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        let state = decode_state(self.inner.state.load(SeqCst));
        if !state.is_open {
            return Poll::Ready(Err(SendError { kind: SendErrorKind::Disconnected }));
        }
        self.poll_unparked(Some(cx)).map(Ok)
    }

    fn same_receiver(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    fn is_connected_to(&self, receiver: &Arc<BoundedInner<T, A>, A>) -> bool {
        Arc::ptr_eq(&self.inner, receiver)
    }

    fn ptr(&self) -> *const BoundedInner<T, A> {
        &*self.inner
    }

    fn is_closed(&self) -> bool {
        !decode_state(self.inner.state.load(SeqCst)).is_open
    }

    fn close_channel(&self) {
        self.inner.set_closed();
        self.inner.recv_task.wake();
    }

    fn poll_unparked(&mut self, cx: Option<&mut Context<'_>>) -> Poll<()> {
        if self.maybe_parked {
            let mut task = self.sender_task.lock();

            if !task.is_parked {
                self.maybe_parked = false;
                return Poll::Ready(());
            }

            task.task = cx.map(|cx| cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> Sender<T, A> {
    pub fn try_send(&mut self, msg: T) -> Result<(), TrySendError<T>> {
        if let Some(inner) = &mut self.0 {
            inner.try_send(msg)
        } else {
            Err(TrySendError { err: SendError { kind: SendErrorKind::Disconnected }, val: msg })
        }
    }

    pub fn start_send(&mut self, msg: T) -> Result<(), SendError> {
        self.try_send(msg).map_err(|e| e.err)
    }

    pub fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        let inner = self.0.as_mut().ok_or(SendError { kind: SendErrorKind::Disconnected })?;
        inner.poll_ready(cx)
    }

    pub fn is_closed(&self) -> bool {
        self.0.as_ref().map(BoundedSenderInner::is_closed).unwrap_or(true)
    }

    pub fn close_channel(&mut self) {
        if let Some(inner) = &mut self.0 {
            inner.close_channel();
        }
    }

    pub fn disconnect(&mut self) {
        self.0 = None;
    }

    pub fn same_receiver(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(inner), Some(other)) => inner.same_receiver(other),
            _ => false,
        }
    }

    pub fn is_connected_to(&self, receiver: &Receiver<T, A>) -> bool {
        match (&self.0, &receiver.inner) {
            (Some(inner), Some(receiver)) => inner.is_connected_to(receiver),
            _ => false,
        }
    }

    pub fn hash_receiver<H: core::hash::Hasher>(&self, hasher: &mut H) {
        use core::hash::Hash;
        let ptr = self.0.as_ref().map(|inner| inner.ptr());
        ptr.hash(hasher);
    }
}

impl<T, A: Allocator + Clone> UnboundedSender<T, A> {
    pub fn poll_ready(&self, _: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        let inner = self.0.as_ref().ok_or(SendError { kind: SendErrorKind::Disconnected })?;
        inner.poll_ready_nb()
    }

    pub fn is_closed(&self) -> bool {
        self.0.as_ref().map(UnboundedSenderInner::is_closed).unwrap_or(true)
    }

    pub fn close_channel(&self) {
        if let Some(inner) = &self.0 {
            inner.close_channel();
        }
    }

    pub fn disconnect(&mut self) {
        self.0 = None;
    }

    fn do_send_nb(&self, msg: T) -> Result<(), TrySendError<T>> {
        if let Some(inner) = &self.0 {
            if inner.inc_num_messages().is_some() {
                inner.queue_push_and_signal(msg);
                return Ok(());
            }
        }
        Err(TrySendError { err: SendError { kind: SendErrorKind::Disconnected }, val: msg })
    }

    pub fn start_send(&mut self, msg: T) -> Result<(), SendError> {
        self.do_send_nb(msg).map_err(|e| e.err)
    }

    pub fn unbounded_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.do_send_nb(msg)
    }

    pub fn same_receiver(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(inner), Some(other)) => inner.same_receiver(other),
            _ => false,
        }
    }

    pub fn is_connected_to(&self, receiver: &UnboundedReceiver<T, A>) -> bool {
        match (&self.0, &receiver.inner) {
            (Some(inner), Some(receiver)) => inner.is_connected_to(receiver),
            _ => false,
        }
    }

    pub fn hash_receiver<H: core::hash::Hasher>(&self, hasher: &mut H) {
        use core::hash::Hash;
        let ptr = self.0.as_ref().map(|inner| inner.ptr());
        ptr.hash(hasher);
    }

    pub fn len(&self) -> usize {
        if let Some(sender) = &self.0 {
            decode_state(sender.inner.state.load(SeqCst)).num_messages
        } else {
            0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> Clone for Sender<T, A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, A: Allocator + Clone> Clone for UnboundedSender<T, A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, A: Allocator + Clone> Clone for UnboundedSenderInner<T, A> {
    fn clone(&self) -> Self {
        let mut curr = self.inner.num_senders.load(SeqCst);

        loop {
            if curr == MAX_BUFFER {
                panic!("cannot clone `Sender` -- too many outstanding senders");
            }

            debug_assert!(curr < MAX_BUFFER);

            let next = curr + 1;
            match self.inner.num_senders.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => return Self { inner: self.inner.clone() },
                Err(actual) => curr = actual,
            }
        }
    }
}

impl<T, A: Allocator + Clone> Clone for BoundedSenderInner<T, A> {
    fn clone(&self) -> Self {
        let mut curr = self.inner.num_senders.load(SeqCst);

        loop {
            if curr == self.inner.max_senders() {
                panic!("cannot clone `Sender` -- too many outstanding senders");
            }

            debug_assert!(curr < self.inner.max_senders());

            let next = curr + 1;
            match self.inner.num_senders.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => {
                    return Self {
                        inner: self.inner.clone(),
                        sender_task: Arc::new_in(
                            Mutex::new(SenderTask::new()),
                            Arc::allocator(&self.inner).clone(),
                        ),
                        maybe_parked: false,
                    };
                }
                Err(actual) => curr = actual,
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> Drop for UnboundedSenderInner<T, A> {
    fn drop(&mut self) {
        let prev = self.inner.num_senders.fetch_sub(1, SeqCst);
        if prev == 1 {
            self.close_channel();
        }
    }
}

impl<T, A: Allocator + Clone> Drop for BoundedSenderInner<T, A> {
    fn drop(&mut self) {
        let prev = self.inner.num_senders.fetch_sub(1, SeqCst);
        if prev == 1 {
            self.close_channel();
        }
    }
}

impl<T, A: Allocator + Clone> fmt::Debug for Sender<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").field("closed", &self.is_closed()).finish()
    }
}

impl<T, A: Allocator + Clone> fmt::Debug for UnboundedSender<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnboundedSender").field("closed", &self.is_closed()).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> Receiver<T, A> {
    pub fn recv(&mut self) -> Recv<'_, Self> {
        Recv::new(self)
    }

    pub fn close(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.set_closed();
            while let Some(task) = unsafe { inner.parked_queue.pop_spin() } {
                task.lock().notify();
            }
        }
    }

    #[deprecated(note = "please use `try_recv` instead")]
    pub fn try_next(&mut self) -> Result<Option<T>, TryRecvError> {
        match self.next_message() {
            Poll::Ready(msg) => Ok(msg),
            Poll::Pending => Err(TryRecvError::Empty),
        }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match self.next_message() {
            Poll::Ready(Some(msg)) => Ok(msg),
            Poll::Ready(None) => Err(TryRecvError::Closed),
            Poll::Pending => Err(TryRecvError::Empty),
        }
    }

    fn next_message(&mut self) -> Poll<Option<T>> {
        let inner = match self.inner.as_mut() {
            None => return Poll::Ready(None),
            Some(inner) => inner,
        };
        match unsafe { inner.message_queue.pop_spin() } {
            Some(msg) => {
                self.unpark_one();
                self.dec_num_messages();
                Poll::Ready(Some(msg))
            }
            None => {
                let state = decode_state(inner.state.load(SeqCst));
                if state.is_closed() {
                    self.inner = None;
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }

    fn unpark_one(&mut self) {
        if let Some(inner) = &mut self.inner {
            if let Some(task) = unsafe { inner.parked_queue.pop_spin() } {
                task.lock().notify();
            }
        }
    }

    fn dec_num_messages(&self) {
        if let Some(inner) = &self.inner {
            inner.state.fetch_sub(1, SeqCst);
        }
    }
}

impl<T, A: Allocator + Clone> Unpin for Receiver<T, A> {}

impl<T, A: Allocator + Clone> FusedStream for Receiver<T, A> {
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}

impl<T, A: Allocator + Clone> Stream for Receiver<T, A> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        match self.next_message() {
            Poll::Ready(msg) => {
                if msg.is_none() {
                    self.inner = None;
                }
                Poll::Ready(msg)
            }
            Poll::Pending => {
                self.inner.as_ref().unwrap().recv_task.register(cx.waker());
                self.next_message()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(inner) = &self.inner {
            decode_state(inner.state.load(SeqCst)).size_hint()
        } else {
            (0, Some(0))
        }
    }
}

impl<T, A: Allocator + Clone> Drop for Receiver<T, A> {
    fn drop(&mut self) {
        self.close();
        if self.inner.is_some() {
            loop {
                match self.next_message() {
                    Poll::Ready(Some(_)) => {}
                    Poll::Ready(None) => break,
                    Poll::Pending => {
                        let state = decode_state(self.inner.as_ref().unwrap().state.load(SeqCst));
                        if state.is_closed() {
                            break;
                        }
                        hint::spin_loop();
                    }
                }
            }
        }
    }
}

impl<T, A: Allocator + Clone> fmt::Debug for Receiver<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let closed = if let Some(ref inner) = self.inner {
            decode_state(inner.state.load(SeqCst)).is_closed()
        } else {
            false
        };
        f.debug_struct("Receiver").field("closed", &closed).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> UnboundedReceiver<T, A> {
    pub fn recv(&mut self) -> Recv<'_, Self> {
        Recv::new(self)
    }

    pub fn close(&mut self) {
        if let Some(inner) = &mut self.inner {
            inner.set_closed();
        }
    }

    #[deprecated(note = "please use `try_recv` instead")]
    pub fn try_next(&mut self) -> Result<Option<T>, TryRecvError> {
        match self.next_message() {
            Poll::Ready(msg) => Ok(msg),
            Poll::Pending => Err(TryRecvError::Empty),
        }
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match self.next_message() {
            Poll::Ready(Some(msg)) => Ok(msg),
            Poll::Ready(None) => Err(TryRecvError::Closed),
            Poll::Pending => Err(TryRecvError::Empty),
        }
    }

    fn next_message(&mut self) -> Poll<Option<T>> {
        let inner = match self.inner.as_mut() {
            None => return Poll::Ready(None),
            Some(inner) => inner,
        };
        match unsafe { inner.message_queue.pop_spin() } {
            Some(msg) => {
                self.dec_num_messages();
                Poll::Ready(Some(msg))
            }
            None => {
                let state = decode_state(inner.state.load(SeqCst));
                if state.is_closed() {
                    self.inner = None;
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }

    fn dec_num_messages(&self) {
        if let Some(inner) = &self.inner {
            inner.state.fetch_sub(1, SeqCst);
        }
    }
}

impl<T, A: Allocator + Clone> FusedStream for UnboundedReceiver<T, A> {
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}

impl<T, A: Allocator + Clone> Stream for UnboundedReceiver<T, A> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        match self.next_message() {
            Poll::Ready(msg) => {
                if msg.is_none() {
                    self.inner = None;
                }
                Poll::Ready(msg)
            }
            Poll::Pending => {
                self.inner.as_ref().unwrap().recv_task.register(cx.waker());
                self.next_message()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(inner) = &self.inner {
            decode_state(inner.state.load(SeqCst)).size_hint()
        } else {
            (0, Some(0))
        }
    }
}

impl<T, A: Allocator + Clone> Drop for UnboundedReceiver<T, A> {
    fn drop(&mut self) {
        self.close();
        if self.inner.is_some() {
            loop {
                match self.next_message() {
                    Poll::Ready(Some(_)) => {}
                    Poll::Ready(None) => break,
                    Poll::Pending => {
                        let state = decode_state(self.inner.as_ref().unwrap().state.load(SeqCst));
                        if state.is_closed() {
                            break;
                        }
                        hint::spin_loop();
                    }
                }
            }
        }
    }
}

impl<T, A: Allocator + Clone> fmt::Debug for UnboundedReceiver<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let closed = if let Some(ref inner) = self.inner {
            decode_state(inner.state.load(SeqCst)).is_closed()
        } else {
            false
        };
        f.debug_struct("UnboundedReceiver").field("closed", &closed).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator + Clone> UnboundedInner<T, A> {
    fn set_closed(&self) {
        let curr = self.state.load(SeqCst);
        if !decode_state(curr).is_open {
            return;
        }
        self.state.fetch_and(!OPEN_MASK, SeqCst);
    }
}

impl<T, A: Allocator + Clone> BoundedInner<T, A> {
    fn max_senders(&self) -> usize {
        MAX_CAPACITY - self.buffer
    }

    fn set_closed(&self) {
        let curr = self.state.load(SeqCst);
        if !decode_state(curr).is_open {
            return;
        }
        self.state.fetch_and(!OPEN_MASK, SeqCst);
    }
}

unsafe impl<T: Send, A: Allocator + Clone + Send + Sync> Send for UnboundedInner<T, A> {}
unsafe impl<T: Send, A: Allocator + Clone + Send + Sync> Sync for UnboundedInner<T, A> {}
unsafe impl<T: Send, A: Allocator + Clone + Send + Sync> Send for BoundedInner<T, A> {}
unsafe impl<T: Send, A: Allocator + Clone + Send + Sync> Sync for BoundedInner<T, A> {}

impl State {
    fn is_closed(&self) -> bool {
        !self.is_open && self.num_messages == 0
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.is_open {
            (self.num_messages, None)
        } else {
            (self.num_messages, Some(self.num_messages))
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<St: ?Sized + Unpin> Unpin for Recv<'_, St> {}

impl<'a, St: ?Sized + Stream + Unpin> Recv<'a, St> {
    fn new(stream: &'a mut St) -> Self {
        Self { stream }
    }
}

impl<St: ?Sized + FusedStream + Unpin> FusedFuture for Recv<'_, St> {
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<St: ?Sized + Stream + Unpin> Future for Recv<'_, St> {
    type Output = Result<St::Item, RecvError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(msg)) => Poll::Ready(Ok(msg)),
            Poll::Ready(None) => Poll::Ready(Err(RecvError)),
            Poll::Pending => Poll::Pending,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

fn decode_state(num: usize) -> State {
    State { is_open: num & OPEN_MASK == OPEN_MASK, num_messages: num & MAX_CAPACITY }
}

fn encode_state(state: &State) -> usize {
    let mut num = state.num_messages;
    if state.is_open {
        num |= OPEN_MASK;
    }
    num
}
