use crate::actor::{
    ActorFuture, ActorHandler, ActorMailboxMessageSender, ActorMessage, ActorMessageSender,
    ActorSendError, ActorWeakMailbox, ActorAllocatorHandler,
};
use alloc::sync::Arc;
use core::fmt::{self, Debug, Formatter};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler + ActorAllocatorHandler,
{
    caller: Arc<dyn ActorMailboxMessageSender<M, H>, H::Allocator>,
}

impl<M, H> Clone for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler + ActorAllocatorHandler,
{
    fn clone(&self) -> Self {
        Self {
            caller: self.caller.clone(),
        }
    }
}

impl<M, H> Debug for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler + ActorAllocatorHandler,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorMailbox").finish()
    }
}

impl<M, H> ActorMessageSender<M> for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler + ActorAllocatorHandler,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async { self.caller.send(message).await }
    }
}

impl<M, H> ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler + ActorAllocatorHandler,
{
    pub fn downgrade(&self) -> ActorWeakMailbox<M, H> {
        ActorWeakMailbox::<M, H>::new(Arc::downgrade(&self.caller))
    }
}
