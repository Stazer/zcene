use crate::actor::{ActorFuture, ActorMessage, ActorMessageSender, ActorSendError};
use async_channel::Sender;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Clone)]
#[Constructor(visibility = pub(super))]
pub struct ActorMessageChannelSender<M>
where
    M: ActorMessage,
{
    sender: Sender<M>,
}

impl<M> ActorMessageSender<M> for ActorMessageChannelSender<M>
where
    M: ActorMessage,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move {
            self.sender
                .send(message)
                .await
                .map_err(|_| ActorSendError::Closed)
        }
    }
}
