use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::{
    ActorUnprivilegedKernelPerspective, ActorUnprivilegedPerspective,
    ActorUnprivilegedUserPerspective,
};
use core::arch::asm;
use zcene_core::actor::{
    ActorFuture, ActorMessage, ActorMessageSender, ActorRecipient, ActorSendError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedRecipient<M, P = ActorUnprivilegedUserPerspective>
where
    M: ActorMessage,
    P: ActorUnprivilegedPerspective,
{
    handle: P::Handle<M>,
}

impl<M, P> ActorUnprivilegedRecipient<M, P>
where
    M: ActorMessage,
    P: ActorUnprivilegedPerspective,
{
    pub fn from_handle(handle: P::Handle<M>) -> Self {
        Self { handle }
    }

    pub fn handle(&self) -> &P::Handle<M> {
        &self.handle
    }

    pub fn into_handle(self) -> P::Handle<M> {
        self.handle
    }
}

impl<M> ActorUnprivilegedRecipient<M, ActorUnprivilegedUserPerspective>
where
    M: ActorMessage,
{
    pub const fn new(index: usize) -> Self {
        Self { handle: index }
    }

    pub const fn index(&self) -> usize {
        self.handle
    }
}

impl<M> ActorUnprivilegedRecipient<M, ActorUnprivilegedKernelPerspective>
where
    M: ActorMessage,
{
    pub fn from_recipient(recipient: ActorRecipient<M, ActorRootEnvironment>) -> Self {
        Self { handle: recipient }
    }
}

impl<M, P> Clone for ActorUnprivilegedRecipient<M, P>
where
    M: ActorMessage,
    P: ActorUnprivilegedPerspective,
{
    fn clone(&self) -> Self {
        Self { handle: self.handle.clone() }
    }
}

impl<M> Copy for ActorUnprivilegedRecipient<M, ActorUnprivilegedUserPerspective> where
    M: ActorMessage
{
}

impl<M> ActorMessageSender<M> for ActorUnprivilegedRecipient<M, ActorUnprivilegedUserPerspective>
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        let index = self.handle;

        async move {
            send_message_copy(index, &message);

            Ok(())
        }
    }
}

impl<M> ActorMessageSender<M> for ActorUnprivilegedRecipient<M, ActorUnprivilegedKernelPerspective>
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.handle.send(message).await }
    }
}

pub(crate) fn send_message_copy<M>(recipient: usize, message: &M) {

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
