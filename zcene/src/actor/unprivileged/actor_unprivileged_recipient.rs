use crate::actor::unprivileged::ActorUnprivilegedSystemCalls;
use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironmentPerspective, ActorUnprivilegedEnvironmentPrivilegedPerspective,
    ActorUnprivilegedEnvironmentUnprivilegedPerspective,
};
use crate::actor::root::ActorRootEnvironment;
use zcene_core::actor::{
    ActorFuture, ActorMessage, ActorMessageSender, ActorRecipient, ActorSendError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A destination for messages of type `M`, from the perspective `P`.
///
/// - unprivileged (default): an index into the recipients the kernel granted to the program.
///   Sending is a system call.
/// - privileged: an ordinary root `ActorRecipient`.
pub struct ActorUnprivilegedRecipient<M, P = ActorUnprivilegedEnvironmentUnprivilegedPerspective>
where
    M: ActorMessage,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    handle: P::Handle<M>,
}

impl<M, P> ActorUnprivilegedRecipient<M, P>
where
    M: ActorMessage,
    P: ActorUnprivilegedEnvironmentPerspective,
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

impl<M> ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentUnprivilegedPerspective>
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

impl<M> ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentPrivilegedPerspective>
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
    P: ActorUnprivilegedEnvironmentPerspective,
{
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl<M> Copy for ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentUnprivilegedPerspective> where
    M: ActorMessage
{
}

impl<M> ActorMessageSender<M>
    for ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentUnprivilegedPerspective>
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        let index = self.handle;

        async move {
            ActorUnprivilegedSystemCalls::send_message_copy(index, &message);

            Ok(())
        }
    }
}

impl<M> ActorMessageSender<M>
    for ActorUnprivilegedRecipient<M, ActorUnprivilegedEnvironmentPrivilegedPerspective>
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.handle.send(message).await }
    }
}
