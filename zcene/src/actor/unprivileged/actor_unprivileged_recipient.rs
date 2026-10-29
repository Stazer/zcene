use core::arch::asm;
use core::marker::PhantomData;
use zcene_core::actor::{ActorFuture, ActorMessage, ActorMessageSender, ActorSendError};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A destination for messages of type `M` as seen from an unprivileged actor: the index into the
/// message handlers it was spawned with. It does not care what kind of actor, or which environment,
/// is behind that handler, only which message type it accepts. Counterpart of `ActorRecipient` for
/// root actors; `ActorUnprivilegedAddress::recipient` turns an address into one.
pub struct ActorUnprivilegedRecipient<M> {
    index: usize,
    marker: PhantomData<fn() -> M>,
}

impl<M> ActorUnprivilegedRecipient<M> {
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            marker: PhantomData,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }
}

impl<M> Clone for ActorUnprivilegedRecipient<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for ActorUnprivilegedRecipient<M> {}

impl<M> ActorMessageSender<M> for ActorUnprivilegedRecipient<M>
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        let index = self.index;

        async move {
            send_message_copy(index, &message);

            Ok(())
        }
    }
}

/// System call 3: the kernel copies the message behind `message` to the handler at `recipient`.
pub(crate) fn send_message_copy<M>(recipient: usize, message: &M) {
    // SAFETY: `syscall` clobbers rcx and r11 (declared via clobber_abi) and, as the kernel resumes
    // us via `sysretq`, may not preserve the callee-saved registers, so they are saved by hand.
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
            in("rsi") recipient,
            in("r8") message as *const M as *const (),
            clobber_abi("C"),
        );
    }
}
