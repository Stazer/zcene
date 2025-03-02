use crate::actor::{
    ActorEnvironment, ActorEnvironmentAllocator, ActorFuture, ActorMailboxMessageSender,
    ActorMessage, ActorMessageSender, ActorSendError, ActorWeakMailbox,
};
use alloc::sync::Arc;
use core::fmt::{self, Debug, Formatter};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    caller: Arc<dyn ActorMailboxMessageSender<M, E>, E::Allocator>,
}

impl<M, E> Clone for ActorMailbox<M, E>
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

impl<M, E> Debug for ActorMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorMailbox").finish()
    }
}

impl<M, E> ActorMessageSender<M> for ActorMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async { self.caller.send(message).await }
    }
}

impl<M, E> ActorMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn downgrade(&self) -> ActorWeakMailbox<M, E> {
        ActorWeakMailbox::<M, E>::new(Arc::downgrade(&self.caller))
    }
}
