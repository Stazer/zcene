// Taken from the concurrent-queue crate (https://github.com/smol-rs/concurrent-queue/)

use core::alloc::Allocator;
use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};
use self::sync::atomic::{self, Ordering};

use self::bounded::Bounded;
use self::single::Single;
use self::sync::busy_wait;
use self::unbounded::Unbounded;

mod bounded;
mod single;
mod sync;
mod unbounded;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ConcurrentQueue<T, A: Allocator + Clone>(Inner<T, A>);

unsafe impl<T: Send, A: Allocator + Clone + Send + Sync> Send for ConcurrentQueue<T, A> {}
unsafe impl<T: Send, A: Allocator + Clone + Send + Sync> Sync for ConcurrentQueue<T, A> {}

impl<T, A: Allocator + Clone> UnwindSafe for ConcurrentQueue<T, A> {}
impl<T, A: Allocator + Clone> RefUnwindSafe for ConcurrentQueue<T, A> {}

#[allow(clippy::large_enum_variant)]
enum Inner<T, A: Allocator + Clone> {
    Single(Single<T>),
    Bounded(Bounded<T, A>),
    Unbounded(Unbounded<T, A>),
}

impl<T, A: Allocator + Clone> ConcurrentQueue<T, A> {
    pub fn bounded_in(cap: usize, alloc: A) -> ConcurrentQueue<T, A> {
        if cap == 1 {
            ConcurrentQueue(Inner::Single(Single::new()))
        } else {
            ConcurrentQueue(Inner::Bounded(Bounded::new_in(cap, alloc)))
        }
    }

    pub fn unbounded_in(alloc: A) -> ConcurrentQueue<T, A> {
        ConcurrentQueue(Inner::Unbounded(Unbounded::new_in(alloc)))
    }

    pub fn push(&self, value: T) -> Result<(), PushError<T>> {
        match &self.0 {
            Inner::Single(q) => q.push(value),
            Inner::Bounded(q) => q.push(value),
            Inner::Unbounded(q) => q.push(value),
        }
    }

    pub fn force_push(&self, value: T) -> Result<Option<T>, ForcePushError<T>> {
        match &self.0 {
            Inner::Single(q) => q.force_push(value),
            Inner::Bounded(q) => q.force_push(value),
            Inner::Unbounded(q) => match q.push(value) {
                Ok(()) => Ok(None),
                Err(PushError::Closed(value)) => Err(ForcePushError(value)),
                Err(PushError::Full(_)) => unreachable!(),
            },
        }
    }

    pub fn pop(&self) -> Result<T, PopError> {
        match &self.0 {
            Inner::Single(q) => q.pop(),
            Inner::Bounded(q) => q.pop(),
            Inner::Unbounded(q) => q.pop(),
        }
    }

    pub fn try_iter(&self) -> TryIter<'_, T, A> {
        TryIter { queue: self }
    }

    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Inner::Single(q) => q.is_empty(),
            Inner::Bounded(q) => q.is_empty(),
            Inner::Unbounded(q) => q.is_empty(),
        }
    }

    pub fn is_full(&self) -> bool {
        match &self.0 {
            Inner::Single(q) => q.is_full(),
            Inner::Bounded(q) => q.is_full(),
            Inner::Unbounded(q) => q.is_full(),
        }
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            Inner::Single(q) => q.len(),
            Inner::Bounded(q) => q.len(),
            Inner::Unbounded(q) => q.len(),
        }
    }

    pub fn capacity(&self) -> Option<usize> {
        match &self.0 {
            Inner::Single(_) => Some(1),
            Inner::Bounded(q) => Some(q.capacity()),
            Inner::Unbounded(_) => None,
        }
    }

    pub fn close(&self) -> bool {
        match &self.0 {
            Inner::Single(q) => q.close(),
            Inner::Bounded(q) => q.close(),
            Inner::Unbounded(q) => q.close(),
        }
    }

    pub fn is_closed(&self) -> bool {
        match &self.0 {
            Inner::Single(q) => q.is_closed(),
            Inner::Bounded(q) => q.is_closed(),
            Inner::Unbounded(q) => q.is_closed(),
        }
    }
}

impl<T, A: Allocator + Clone> fmt::Debug for ConcurrentQueue<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConcurrentQueue")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("is_closed", &self.is_closed())
            .finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct TryIter<'a, T, A: Allocator + Clone> {
    queue: &'a ConcurrentQueue<T, A>,
}

impl<T, A: Allocator + Clone> Clone for TryIter<'_, T, A> {
    fn clone(&self) -> Self {
        TryIter { queue: self.queue }
    }
}

impl<T, A: Allocator + Clone> fmt::Debug for TryIter<'_, T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TryIter").field(&self.queue).finish()
    }
}

impl<T, A: Allocator + Clone> Iterator for TryIter<'_, T, A> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop().ok()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PopError {
    Empty,
    Closed,
}

impl PopError {
    pub fn is_empty(&self) -> bool {
        matches!(self, PopError::Empty)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, PopError::Closed)
    }
}

impl fmt::Debug for PopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::Empty => write!(f, "Empty"),
            PopError::Closed => write!(f, "Closed"),
        }
    }
}

impl fmt::Display for PopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::Empty => write!(f, "Empty"),
            PopError::Closed => write!(f, "Closed"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PushError<T> {
    Full(T),
    Closed(T),
}

impl<T> PushError<T> {
    pub fn into_inner(self) -> T {
        match self {
            PushError::Full(t) => t,
            PushError::Closed(t) => t,
        }
    }

    pub fn is_full(&self) -> bool {
        matches!(self, PushError::Full(_))
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, PushError::Closed(_))
    }
}

impl<T: fmt::Debug> fmt::Debug for PushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PushError::Full(t) => f.debug_tuple("Full").field(t).finish(),
            PushError::Closed(t) => f.debug_tuple("Closed").field(t).finish(),
        }
    }
}

impl<T> fmt::Display for PushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PushError::Full(_) => write!(f, "Full"),
            PushError::Closed(_) => write!(f, "Closed"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ForcePushError<T>(pub T);

impl<T> ForcePushError<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for ForcePushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ForcePushError").field(&self.0).finish()
    }
}

impl<T> fmt::Display for ForcePushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Closed")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[inline]
fn full_fence() {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use core::{arch::asm, cell::UnsafeCell};
        let a = UnsafeCell::new(0_usize);
        unsafe {
            #[cfg(target_pointer_width = "64")]
            asm!("lock not qword ptr [{0}]", in(reg) a.get(), options(nostack, preserves_flags));
            #[cfg(target_pointer_width = "32")]
            asm!("lock not dword ptr [{0:e}]", in(reg) a.get(), options(nostack, preserves_flags));
        }
        return;
    }
    #[allow(unreachable_code)]
    {
        atomic::fence(Ordering::SeqCst);
    }
}
