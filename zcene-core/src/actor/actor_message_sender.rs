use crate::actor::{ActorCommonBounds, ActorFuture, ActorMessage, ActorSendError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorMessageSender<M>: ActorCommonBounds
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>>;
}
