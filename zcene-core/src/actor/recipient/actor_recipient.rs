use crate::actor::{
    ActorBoxFuture, ActorEnvironment, ActorEnvironmentAllocator, ActorFuture, ActorMessage,
    ActorMessageChannelSender, ActorMessageEnvelope, ActorMessageSender,
    ActorRecipientMessageSender, ActorSendError, ActorWeakRecipient,
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::alloc::AllocError;
use core::fmt::{self, Debug, Formatter};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    caller: Arc<dyn ActorRecipientMessageSender<M, E> + Send + Sync, E::Allocator>,
}

impl<M, E> Clone for ActorRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn clone(&self) -> Self {
        Self {
            caller: self.caller.clone(),
        }
    }
}

impl<M, E> Debug for ActorRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorRecipient").finish()
    }
}

impl<M, E> ActorMessageSender<M> for ActorRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async { self.caller.send(message).await }
    }
}

impl<M, E> ActorRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn downgrade(&self) -> ActorWeakRecipient<M, E> {
        ActorWeakRecipient::<M, E>::new(Arc::downgrade(&self.caller))
    }

    /// A recipient that delivers straight into `sender`'s channel, for executors that are not
    /// driven by an `Actor<E>` implementation of their own (so there is no address to derive it
    /// from).
    pub fn from_channel(
        sender: ActorMessageChannelSender<ActorMessageEnvelope<M, E>, E::Allocator>,
        allocator: E::Allocator,
    ) -> Result<Self, AllocError> {
        Ok(Self::new(Arc::try_new_in(
            ActorChannelRecipientMessageSender { sender, allocator: allocator.clone() },
            allocator,
        )?))
    }
}

struct ActorChannelRecipientMessageSender<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    sender: ActorMessageChannelSender<ActorMessageEnvelope<M, E>, E::Allocator>,
    allocator: E::Allocator,
}

impl<M, E> ActorRecipientMessageSender<M, E> for ActorChannelRecipientMessageSender<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, E> {
        Box::pin_in(
            self.sender.transmit(ActorMessageEnvelope::Message(message)),
            self.allocator.clone(),
        )
    }
}
