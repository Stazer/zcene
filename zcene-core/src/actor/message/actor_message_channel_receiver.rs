use crate::actor::ActorMessage;
use async_channel::Receiver;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[Constructor(visibility = pub(super))]
pub struct ActorMessageChannelReceiver<M>
where
    M: ActorMessage,
{
    receiver: Receiver<M>,
}

impl<M> ActorMessageChannelReceiver<M>
where
    M: ActorMessage,
{
    pub async fn receive(&self) -> Option<M> {
        self.receiver.recv().await.ok()
    }
}
