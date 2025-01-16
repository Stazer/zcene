use crate::actor::{
    ActorFuture, ActorHandler, ActorMailboxMessageSender, ActorMessage, ActorMessageSender,
    ActorSendError,
};
use alloc::sync::Arc;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    caller: Arc<dyn ActorMailboxMessageSender<M, H>, H::Allocator>,
}

impl<M, H> Clone for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    fn clone(&self) -> Self {
        Self {
            caller: self.caller.clone(),
        }
    }
}

impl<M, H> ActorMessageSender<M> for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async { self.caller.send(message).await }
    }
}
