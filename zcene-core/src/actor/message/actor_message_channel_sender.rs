use crate::actor::{ActorAllocator, ActorFuture, ActorMessage, ActorMessageSender, ActorSendError};
use futures_channel::mpsc::UnboundedSender;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[Constructor(visibility = pub(super))]
pub struct ActorMessageChannelSender<M, A>
where
    M: ActorMessage,
    A: ActorAllocator,
{
    sender: UnboundedSender<M, A>,
}

impl<M, A> Clone for ActorMessageChannelSender<M, A>
where
    M: ActorMessage,
    A: ActorAllocator,
{
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone() }
    }
}

impl<M, A> ActorMessageSender<M> for ActorMessageChannelSender<M, A>
where
    M: ActorMessage,
    A: ActorAllocator,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move {
            self.sender
                .unbounded_send(message)
                .map_err(|_| ActorSendError::Closed)
        }
    }
}
