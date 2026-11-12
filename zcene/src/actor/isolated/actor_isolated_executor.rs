use crate::actor::isolated::{
    ActorIsolatedImage, ActorIsolatedMemory, ActorIsolatedMessage, ActorIsolatedMessageHandlers,
    ActorIsolatedPhase,
};
use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::ActorUnprivilegedSystemCallKind;
use crate::common::memory::allocator::ErasedAllocator;
use crate::platform::{ActivePlatform, Platform, UnprivilegedEvent};
use alloc::boxed::Box;
use core::fmt::Write;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use zcene_core::actor::{
    ActorEnvironmentAllocator, ActorEnvironmentReference, ActorMessageChannelReceiver,
    ActorMessageEnvelope, ActorRecipient, ActorSpawnError,
};
use zcene_core::future::r#yield;

////////////////////////////////////////////////////////////////////////////////////////////////////

type P = ActivePlatform;

type PlatformEvent = UnprivilegedEvent<
    <P as Platform>::UnprivilegedSystemCallContext,
    <P as Platform>::UnprivilegedPreemptionContext,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhaseOutcome {
    Completed,
    Faulted,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedExecutorInner<M>
where
    M: ActorIsolatedMessage,
{
    environment: ActorEnvironmentReference<ActorRootEnvironment>,
    memory: ActorIsolatedMemory<M>,
    receiver: ActorMessageChannelReceiver<ActorMessageEnvelope<M, ActorRootEnvironment>, ErasedAllocator>,
    _recipient: ActorRecipient<M, ActorRootEnvironment>,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    message_handlers: ActorIsolatedMessageHandlers,
}

impl<M> ActorIsolatedExecutorInner<M>
where
    M: ActorIsolatedMessage,
{
    async fn run(mut self) {
        if self.run_phase(ActorIsolatedPhase::Create, None).await == PhaseOutcome::Faulted {
            return;
        }

        while let Some(envelope) = self.receiver.receive().await {
            match envelope {
                ActorMessageEnvelope::Message(message) => {
                    let address = self.memory.store_message(message);

                    if self.run_phase(ActorIsolatedPhase::Handle, Some(address)).await
                        == PhaseOutcome::Faulted
                    {
                        return;
                    }
                }
                ActorMessageEnvelope::Migrate(_) => {}
            }
        }

        let _ = self.run_phase(ActorIsolatedPhase::Destroy, None).await;
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
    async fn run_phase(&mut self, phase: ActorIsolatedPhase, message: Option<u64>) -> PhaseOutcome {
        let mut outcome = PhaseOutcome::Completed;
        let mut event: Option<PlatformEvent> = None;

        self.enable_deadline();

        P::execute_in_unprivileged(
            phase as u64 as *mut (),
            message.map_or(core::ptr::null(), |address| address as *const ()),
            &mut event,
            self.memory.stack_top(),
            unsafe {
                core::mem::transmute::<usize, extern "C" fn(*mut (), *const ()) -> !>(
                    self.memory.entry() as usize,
                )
            },
            Some(self.memory.address_space()),
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
                            P::resume_after_system_call(
                                &syscall.context,
                                &mut event,
                                Some(self.memory.address_space()),
                            );
                        }

                        ActorUnprivilegedSystemCallKind::Preempt => {
                            self.disable_deadline();
                            r#yield().await;
                            self.enable_deadline();
                            P::resume_after_system_call(
                                &syscall.context,
                                &mut event,
                                Some(self.memory.address_space()),
                            );
                        }

                        ActorUnprivilegedSystemCallKind::Complete => break,

                        ActorUnprivilegedSystemCallKind::SendMessageCopy { recipient, message } => {
                            let user_address = message as usize as u64;

                            if let Some(handler) = self.message_handlers.get(recipient) {
                                if !self.memory.contains(user_address, handler.message_layout()) {
                                    let _ = writeln!(
                                        self.environment.logger(),
                                        "isolated actor passed an invalid message pointer {:#x}",
                                        user_address,
                                    );
                                    outcome = PhaseOutcome::Faulted;
                                    break;
                                }

                                let source = self.memory.kernel_address(user_address);
                                let allocator = self.environment.allocator().clone();
                                let _ = handler.send(&allocator, source as *const ()).await;
                            }

                            P::resume_after_system_call(
                                &syscall.context,
                                &mut event,
                                Some(self.memory.address_space()),
                            );
                        }

                        ActorUnprivilegedSystemCallKind::Unknown(identifier) => {
                            let _ = writeln!(
                                self.environment.logger(),
                                "isolated actor made unknown system call {}",
                                identifier,
                            );
                            outcome = PhaseOutcome::Faulted;
                            break;
                        }
                    }
                }

                Some(UnprivilegedEvent::Preemption(context)) => {
                    self.environment.platform().notify_end_of_interrupt();
                    self.disable_deadline();
                    r#yield().await;
                    self.enable_deadline();
                    P::resume_after_preemption(
                        &context,
                        &mut event,
                        Some(self.memory.address_space()),
                    );
                }

                Some(UnprivilegedEvent::Exception(exception)) => {
                    let _ = writeln!(
                        self.environment.logger(),
                        "isolated actor faulted: {:x?}",
                        exception,
                    );
                    outcome = PhaseOutcome::Faulted;
                    break;
                }
            }
        }

        self.disable_deadline();

        outcome
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedExecutor<M>
where
    M: ActorIsolatedMessage,
{
    future: Pin<Box<dyn Future<Output = ()> + Send + 'static, ErasedAllocator>>,
    _marker: PhantomData<fn() -> M>,
}

impl<M> ActorIsolatedExecutor<M>
where
    M: ActorIsolatedMessage,
{
    pub fn new(
        environment: ActorEnvironmentReference<ActorRootEnvironment>,
        image: ActorIsolatedImage,
        receiver: ActorMessageChannelReceiver<ActorMessageEnvelope<M, ActorRootEnvironment>, ErasedAllocator>,
        recipient: ActorRecipient<M, ActorRootEnvironment>,
        deadline_in_milliseconds: Option<NonZero<usize>>,
        message_handlers: ActorIsolatedMessageHandlers,
    ) -> Result<Self, ActorSpawnError> {
        let allocator = environment.allocator().clone();

        let inner = ActorIsolatedExecutorInner::<M> {
            memory: ActorIsolatedMemory::new(environment.platform(), &image)?,
            environment,
            receiver,
            _recipient: recipient,
            deadline_in_milliseconds,
            message_handlers,
        };

        Ok(Self {
            future: Box::pin_in(inner.run(), allocator),
            _marker: PhantomData,
        })
    }
}

impl<M> Future for ActorIsolatedExecutor<M>
where
    M: ActorIsolatedMessage,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
