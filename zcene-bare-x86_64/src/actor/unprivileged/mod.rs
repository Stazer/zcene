mod executor;

pub use executor::*;

use core::marker::PhantomData;
use zcene_core::actor::{ActorSendError, Actor, ActorHandler, ActorMessage, ActorAddress, ActorMessageSender, ActorFuture};

use core::arch::asm;

pub struct ActorUnprivilegedHandler;

impl ActorHandler for ActorUnprivilegedHandler {
    type Address<A> = ActorUnprivilegedAddress<A>
    where
        A: Actor<Self>;
    type CreateContext = ();
    type HandleContext<M> = ()
    where
        M: ActorMessage;
    type DestroyContext = ();
}

pub struct ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedHandler>,
{
    descriptor: usize,
    marker: PhantomData<A::Message>,
}

impl<A> ActorAddress<A, ActorUnprivilegedHandler> for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedHandler>,
{
}

impl<A> ActorMessageSender<A::Message> for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedHandler>,
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
                    in ("rsi") &message,
                    in ("rdx") size_of::<A::Message>(),
                    clobber_abi("C"),
                );

            }

            Ok(())
        }
    }
}
