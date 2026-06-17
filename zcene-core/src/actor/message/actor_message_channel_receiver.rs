use crate::actor::{ActorAllocator, ActorMessage};
use crate::channel::mpsc::UnboundedReceiver;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[Constructor(visibility = pub(super))]
pub struct ActorMessageChannelReceiver<M, A>
where
    M: ActorMessage,
    A: ActorAllocator,
{
    receiver: UnboundedReceiver<M, A>,
}

impl<M, A> ActorMessageChannelReceiver<M, A>
where
    M: ActorMessage,
    A: ActorAllocator,
{
    pub async fn receive(&mut self) -> Option<M> {
        self.receiver.recv().await.ok()
    }
}
