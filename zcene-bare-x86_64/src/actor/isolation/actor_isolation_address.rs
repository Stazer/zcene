use crate::actor::ActorIsolationEnvironment;
use core::arch::asm;
use core::marker::PhantomData;
use zcene_core::actor::{
    Actor, ActorAddress, ActorCommonHandleContext, ActorEnvironment, ActorFuture, ActorMessage,
    ActorMessageSender, ActorSendError,
};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorIsolationAddress<A>
where
    A: Actor<ActorIsolationEnvironment>,
{
    descriptor: usize,
    #[Constructor(default)]
    marker: PhantomData<A::Message>,
}

impl<A> Clone for ActorIsolationAddress<A>
where
    A: Actor<ActorIsolationEnvironment>,
{
    fn clone(&self) -> Self {
        Self::new(self.descriptor)
    }
}

impl<A> ActorAddress<A, ActorIsolationEnvironment> for ActorIsolationAddress<A> where
    A: Actor<ActorIsolationEnvironment>,
{
}

impl<A> ActorMessageSender<A::Message> for ActorIsolationAddress<A>
where
    A: Actor<ActorIsolationEnvironment>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move {
            unsafe {
                asm!(
                    "push rbx",
                    "push rbp",
                    "push r12",
                    "push r13",
                    "push r14",
                    "push r15",
                    "mov rdi, 3",
                    "syscall",
                    "pop r15",
                    "pop r14",
                    "pop r13",
                    "pop r12",
                    "pop rbp",
                    "pop rbx",
                    in ("rdi") 3,
                    in ("rsi") &message,
                    in ("rdx") size_of::<A::Message>(),
                    clobber_abi("C"),
                );
            }

            Ok(())
        }
    }
}
