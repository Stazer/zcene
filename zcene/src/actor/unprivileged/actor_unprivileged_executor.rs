use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::{ActorUnprivilegedEnvironment, ActorUnprivilegedMessageHandler};
use crate::common::memory::allocator::LeakingHeapMemoryAllocator;
use crate::actor::unprivileged::ActorUnprivilegedSystemCallKind;
use crate::platform::{Platform, UnprivilegedEvent};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use zcene_core::actor::{
    Actor, ActorCommonHandleContext, ActorEnvironmentAllocator, ActorEnvironmentReference,
    ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use zcene_core::future::r#yield;

////////////////////////////////////////////////////////////////////////////////////////////////////

type PlatformEvent<P> = UnprivilegedEvent<
    <P as Platform>::UnprivilegedSystemCallContext,
    <P as Platform>::UnprivilegedPreemptionContext,
>;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedExecutorInner<AI, AR, H, P>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: Platform,
{
    environment: ActorEnvironmentReference<ActorRootEnvironment<H, P>>,
    actor: Box<AI>,
    receiver: ActorMessageChannelReceiver<
        AR::Message,
        <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
    >,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    message_handlers: Vec<
        Box<
            dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment<H, P>> + Send + Sync,
            <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
        >,
        <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
    >,
    marker: PhantomData<(AR, H)>,
}

impl<AI, AR, H, P> ActorUnprivilegedExecutorInner<AI, AR, H, P>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: Platform,
{
    fn new(
        environment: ActorEnvironmentReference<ActorRootEnvironment<H, P>>,
        actor: Box<AI>,
        receiver: ActorMessageChannelReceiver<
        AR::Message,
        <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
    >,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: Vec<
            Box<
                dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment<H, P>> + Send + Sync,
                <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
            >,
            <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
        >,
    ) -> Self {
        Self {
            environment,
            actor,
            receiver,
            deadline_in_milliseconds,
            message_handlers,
            marker: PhantomData,
        }
    }

    async fn run(mut self) {
        self.run_phase(|actor, event, stack| {
            P::execute_in_unprivileged(
                actor as *mut (),
                core::ptr::null(),
                event,
                stack,
                unsafe {
                    core::mem::transmute::<
                        extern "C" fn(*mut AI, *const ()) -> !,
                        extern "C" fn(*mut (), *const ()) -> !,
                    >(Self::create_entry)
                },
            );
        })
        .await;

        loop {
            let message = match self.receiver.receive().await {
                None => break,
                Some(m) => m,
            };

            self.run_phase(|actor, event, stack| {
                P::execute_in_unprivileged(
                    actor as *mut (),
                    &message as *const AI::Message as *const (),
                    event,
                    stack,
                    unsafe {
                        core::mem::transmute::<
                            extern "C" fn(*mut AI, *const AI::Message) -> !,
                            extern "C" fn(*mut (), *const ()) -> !,
                        >(Self::handle_entry)
                    },
                );
            })
            .await;
        }

        self.run_phase(|actor, event, stack| {
            P::execute_in_unprivileged(
                actor as *mut (),
                core::ptr::null(),
                event,
                stack,
                unsafe {
                    core::mem::transmute::<
                        extern "C" fn(*mut AI, *const ()) -> !,
                        extern "C" fn(*mut (), *const ()) -> !,
                    >(Self::destroy_entry)
                },
            );
        })
        .await;
    }

    fn enable_deadline(&self) {
        if let Some(ms) = self.deadline_in_milliseconds {
            self.environment
                .platform()
                .set_preemption_deadline(Duration::from_millis(usize::from(ms) as u64));
        }
    }

    #[inline(never)]
    async fn run_phase<F>(&mut self, execute: F)
    where
        F: FnOnce(*mut AI, &mut Option<PlatformEvent<P>>, u64),
    {
        let mut event: Option<PlatformEvent<P>> = None;

        let stack = self.environment.platform().allocate_unprivileged_stack().unwrap();

        self.enable_deadline();

        execute(Box::as_mut_ptr(&mut self.actor), &mut event, stack);

        loop {
            match event.take() {
                None => break,

                Some(UnprivilegedEvent::SystemCall(syscall)) => {
                    let kind = ActorUnprivilegedSystemCallKind::parse(
                        syscall.identifier,
                        syscall.argument0,
                        syscall.argument1,
                    );

                    match kind {
                        ActorUnprivilegedSystemCallKind::Continue => {
                            P::resume_after_system_call(&syscall.context, &mut event);
                        }

                        ActorUnprivilegedSystemCallKind::Preempt => {
                            r#yield().await;
                            self.enable_deadline();
                            P::resume_after_system_call(&syscall.context, &mut event);
                        }

                        ActorUnprivilegedSystemCallKind::Complete => break,

                        ActorUnprivilegedSystemCallKind::SendMessageCopy { mailbox, message } => {
                            if let Some(handler) = self.message_handlers.get(mailbox) {
                                let allocator = self.environment.allocator().clone();
                                let _ = handler.send(&allocator, message).await;
                            }
                            P::resume_after_system_call(&syscall.context, &mut event);
                        }

                        ActorUnprivilegedSystemCallKind::Unknown(_) => break,
                    }
                }

                Some(UnprivilegedEvent::Preemption(context)) => {
                    self.environment.platform().notify_end_of_interrupt();
                    r#yield().await;
                    self.enable_deadline();
                    P::resume_after_preemption(&context, &mut event);
                }

                Some(UnprivilegedEvent::Exception) => break,
            }
        }
    }

    extern "C" fn create_entry(actor: *mut AI, _message: *const ()) -> ! {
        let mut actor = unsafe { Box::from_raw_in(actor, LeakingHeapMemoryAllocator) };
        let mut ctx = Context::from_waker(Waker::noop());
        let mut fut = pin!(actor.create(()));
        let _result = match fut.as_mut().poll(&mut ctx) {
            Poll::Pending => todo!(),
            Poll::Ready(r) => r,
        };
        P::complete_unprivileged_execution()
    }

    extern "C" fn handle_entry(actor: *mut AI, message: *const AI::Message) -> ! {
        let mut actor = unsafe { Box::from_raw_in(actor, LeakingHeapMemoryAllocator) };
        let message = unsafe { message.as_ref().unwrap() };
        let mut ctx = Context::from_waker(Waker::noop());
        let mut fut = pin!(actor.handle(ActorCommonHandleContext::new(message.clone())));
        let _result = match fut.as_mut().poll(&mut ctx) {
            Poll::Pending => todo!(),
            Poll::Ready(r) => r,
        };
        P::complete_unprivileged_execution()
    }

    extern "C" fn destroy_entry(actor: *mut AI, _message: *const ()) -> ! {
        let mut actor = unsafe { Box::from_raw_in(actor, LeakingHeapMemoryAllocator) };
        let mut ctx = Context::from_waker(Waker::noop());
        let mut fut = pin!(actor.destroy(()));
        let _result = match fut.as_mut().poll(&mut ctx) {
            Poll::Pending => todo!(),
            Poll::Ready(r) => r,
        };
        P::complete_unprivileged_execution()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedExecutor<AI, AR, H, P>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: Platform,
{
    future: Pin<
        Box<
            dyn Future<Output = ()> + Send + 'static,
            <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
        >,
    >,
    _marker: PhantomData<fn() -> (AI, AR, H, P)>,
}

impl<AI, AR, H, P> ActorUnprivilegedExecutor<AI, AR, H, P>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: Platform,
    ActorUnprivilegedExecutorInner<AI, AR, H, P>: Send + 'static,
{
    pub fn new(
        environment: ActorEnvironmentReference<ActorRootEnvironment<H, P>>,
        actor: Box<AI>,
        receiver: ActorMessageChannelReceiver<
        AR::Message,
        <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
    >,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: Vec<
            Box<
                dyn ActorUnprivilegedMessageHandler<ActorRootEnvironment<H, P>> + Send + Sync,
                <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
            >,
            <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
        >,
    ) -> Self {
        let allocator = environment.allocator().clone();

        let inner = ActorUnprivilegedExecutorInner::<AI, AR, H, P>::new(
            environment,
            actor,
            receiver,
            deadline_in_milliseconds,
            message_handlers,
        );
        Self {
            future: Box::pin_in(inner.run(), allocator),
            _marker: PhantomData,
        }
    }
}

impl<AI, AR, H, P> Future for ActorUnprivilegedExecutor<AI, AR, H, P>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: Platform,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
