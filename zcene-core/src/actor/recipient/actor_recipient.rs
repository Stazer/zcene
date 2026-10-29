use crate::actor::{
    ActorEnvironment, ActorEnvironmentAllocator, ActorFuture, ActorRecipientMessageSender,
    ActorMessage, ActorMessageSender, ActorSendError, ActorWeakRecipient,
};
use alloc::sync::Arc;
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
}
