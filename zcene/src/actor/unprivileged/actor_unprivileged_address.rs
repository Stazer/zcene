use crate::actor::unprivileged::ActorUnprivilegedEnvironment;
use core::arch::asm;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorAddress, ActorFuture, ActorMessageSender, ActorSendError};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    mailbox: usize,
    #[Constructor(default)]
    marker: PhantomData<A::Message>,
}

impl<A> Clone for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    fn clone(&self) -> Self {
        Self::new(self.mailbox)
    }
}

impl<A> ActorAddress<A, ActorUnprivilegedEnvironment> for ActorUnprivilegedAddress<A> where
    A: Actor<ActorUnprivilegedEnvironment>
{
}

impl<A> ActorMessageSender<A::Message> for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        let mailbox = self.mailbox;

        async move {
            // Syscall 3 = SendMessageCopy: rdi=3, rsi=mailbox, r8=message_ptr
            unsafe {
                asm!(
                    "push rbx",
                    "push rbp",
                    "push r12",
                    "push r13",
                    "push r14",
                    "push r15",
                    "syscall",
                    "pop r15",
                    "pop r14",
                    "pop r13",
                    "pop r12",
                    "pop rbp",
                    "pop rbx",
                    in("rdi") 3usize,
                    in("rsi") mailbox,
                    in("r8") &message as *const A::Message as *const (),
                    clobber_abi("C"),
                );
            }

            Ok(())
        }
    }
}
