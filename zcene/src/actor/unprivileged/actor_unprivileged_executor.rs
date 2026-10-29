use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironment, ActorUnprivilegedMessageHandlers,
    ActorUnprivilegedSystemCallKind,
};
use crate::common::memory::allocator::ErasedAllocator;
use crate::platform::{ActivePlatform, Platform, UnprivilegedEvent};
use alloc::boxed::Box;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use zcene_core::actor::{
    Actor, ActorCommonHandleContext, ActorEnvironmentAllocator, ActorEnvironmentReference,
    ActorMailbox, ActorMessageChannelReceiver, ActorMessageEnvelope,
};
use zcene_core::future::r#yield;

////////////////////////////////////////////////////////////////////////////////////////////////////

type P = ActivePlatform;

type PlatformEvent = UnprivilegedEvent<
    <P as Platform>::UnprivilegedSystemCallContext,
    <P as Platform>::UnprivilegedPreemptionContext,
>;

type ActorUnprivilegedReceiver<AI> = ActorMessageChannelReceiver<
    ActorMessageEnvelope<<AI as Actor<ActorUnprivilegedEnvironment>>::Message, ActorRootEnvironment>,
    ErasedAllocator,
>;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Stand-in for an unprivileged actor on the root side, so it can be addressed through a regular
/// `ActorMessageChannelAddress`. It is never instantiated or run.
pub struct ActorUnprivilegedRootProxy<AI>(PhantomData<fn() -> AI>);

impl<AI> Actor<ActorRootEnvironment> for ActorUnprivilegedRootProxy<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    type Message = AI::Message;
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedExecutorInner<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    environment: ActorEnvironmentReference<ActorRootEnvironment>,
    actor: Box<AI, ErasedAllocator>,
    receiver: ActorUnprivilegedReceiver<AI>,
    // Held for the task's lifetime so discovery's weak entry stays upgradable while running.
    _mailbox: ActorMailbox<AI::Message, ActorRootEnvironment>,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    message_handlers: ActorUnprivilegedMessageHandlers,
}

impl<AI> ActorUnprivilegedExecutorInner<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    async fn run(mut self) {
        // One stack per actor, reused by every phase: a phase always runs to completion before
        // the next one starts, so nothing on it is live in between.
        let Some(stack) = self.environment.platform().allocate_unprivileged_stack() else {
            return;
        };

        self.run_phase(stack, None, Self::create_entry)
            .await;

        while let Some(envelope) = self.receiver.receive().await {
            match envelope {
                ActorMessageEnvelope::Message(message) => {
                    self.run_phase(stack, Some(&message), Self::handle_entry)
                        .await;
                }
                // Migration swaps in a root actor, which cannot run unprivileged.
                ActorMessageEnvelope::Migrate(_) => {}
            }
        }

        self.run_phase(stack, None, Self::destroy_entry)
            .await;
    }

    fn enable_deadline(&self) {
        if let Some(ms) = self.deadline_in_milliseconds {
            self.environment
                .platform()
                .set_preemption_deadline(Duration::from_millis(usize::from(ms) as u64));
        }
    }

    fn disable_deadline(&self) {
        if self.deadline_in_milliseconds.is_some() {
            self.environment.platform().clear_preemption_deadline();
        }
    }

    #[inline(never)]
    async fn run_phase(
        &mut self,
        stack: u64,
        message: Option<&AI::Message>,
        entry: extern "C" fn(*mut AI, *const ()) -> !,
    ) {
        let mut event: Option<PlatformEvent> = None;

        self.enable_deadline();

        P::execute_in_unprivileged(
            Box::as_mut_ptr(&mut self.actor) as *mut (),
            message.map_or(core::ptr::null(), |message| message as *const AI::Message as *const ()),
            &mut event,
            stack,
            unsafe {
                core::mem::transmute::<
                    extern "C" fn(*mut AI, *const ()) -> !,
                    extern "C" fn(*mut (), *const ()) -> !,
                >(entry)
            },
        );

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
                            self.disable_deadline();
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
                    self.disable_deadline();
                    r#yield().await;
                    self.enable_deadline();
                    P::resume_after_preemption(&context, &mut event);
                }

                Some(UnprivilegedEvent::Exception) => break,
            }
        }

        self.disable_deadline();
    }

    // The entry points below run in ring 3. They borrow the actor instead of taking ownership:
    // it lives across phases and is owned (and eventually dropped) by the executor.

    extern "C" fn create_entry(actor: *mut AI, _message: *const ()) -> ! {
        let actor = unsafe { &mut *actor };
        let _ = Self::poll_to_completion(actor.create(()));
        P::complete_unprivileged_execution()
    }

    extern "C" fn handle_entry(actor: *mut AI, message: *const ()) -> ! {
        let actor = unsafe { &mut *actor };
        let message = unsafe { &*(message as *const AI::Message) };
        let _ = Self::poll_to_completion(actor.handle(ActorCommonHandleContext::new(message.clone())));
        P::complete_unprivileged_execution()
    }

    extern "C" fn destroy_entry(actor: *mut AI, _message: *const ()) -> ! {
        let actor = unsafe { &mut *actor };
        let _ = Self::poll_to_completion(actor.destroy(()));
        P::complete_unprivileged_execution()
    }

    /// Unprivileged actors have no waker infrastructure yet, so their futures must complete on
    /// the first poll. Waiting has to go through a system call instead.
    fn poll_to_completion<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        match pin!(future).poll(&mut context) {
            Poll::Ready(result) => result,
            Poll::Pending => todo!("unprivileged actor futures must not return Pending"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedExecutor<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    future: Pin<Box<dyn Future<Output = ()> + Send + 'static, ErasedAllocator>>,
    _marker: PhantomData<fn() -> AI>,
}

impl<AI> ActorUnprivilegedExecutor<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
    ActorUnprivilegedExecutorInner<AI>: Send + 'static,
{
    pub fn new(
        environment: ActorEnvironmentReference<ActorRootEnvironment>,
        actor: AI,
        receiver: ActorUnprivilegedReceiver<AI>,
        mailbox: ActorMailbox<AI::Message, ActorRootEnvironment>,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: ActorUnprivilegedMessageHandlers,
    ) -> Self {
        let allocator = environment.allocator().clone();

        let inner = ActorUnprivilegedExecutorInner::<AI> {
            actor: Box::new_in(actor, allocator.clone()),
            environment,
            receiver,
            _mailbox: mailbox,
            deadline_in_milliseconds,
            message_handlers,
        };

        Self {
            future: Box::pin_in(inner.run(), allocator),
            _marker: PhantomData,
        }
    }
}

impl<AI> Future for ActorUnprivilegedExecutor<AI>
where
    AI: Actor<ActorUnprivilegedEnvironment>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
