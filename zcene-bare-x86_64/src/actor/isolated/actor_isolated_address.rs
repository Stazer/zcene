use core::arch::asm;
use core::marker::PhantomData;
use zcene_core::actor::{
    Actor, ActorAddress, ActorCommonHandleContext, ActorEnvironment, ActorFuture, ActorMessage,
    ActorMessageSender, ActorSendError,
};
use ztd::Constructor;
use crate::actor::ActorIsolatedEnvironment;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorIsolatedAddress<A>
where
    A: Actor<ActorIsolatedEnvironment>,
{
    descriptor: usize,
    #[Constructor(default)]
    marker: PhantomData<A::Message>,
}

impl<A> Clone for ActorIsolatedAddress<A>
where
    A: Actor<ActorIsolatedEnvironment>,
{
    fn clone(&self) -> Self {
        Self::new(self.descriptor)
    }
}

impl<A> ActorAddress<A, ActorIsolatedEnvironment> for ActorIsolatedAddress<A> where
    A: Actor<ActorIsolatedEnvironment>
{
}

impl<A> ActorMessageSender<A::Message> for ActorIsolatedAddress<A>
where
    A: Actor<ActorIsolatedEnvironment>,
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
