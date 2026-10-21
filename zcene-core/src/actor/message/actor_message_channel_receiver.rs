use crate::actor::{ActorAllocator, ActorCommonBounds};
use crate::channel::mpsc::UnboundedReceiver;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[Constructor(visibility = pub(super))]
pub struct ActorMessageChannelReceiver<M, A>
where
    M: ActorCommonBounds,
    A: ActorAllocator,
{
    receiver: UnboundedReceiver<M, A>,
}

impl<M, A> ActorMessageChannelReceiver<M, A>
where
    M: ActorCommonBounds,
    A: ActorAllocator,
{
    pub async fn receive(&mut self) -> Option<M> {
        self.receiver.recv().await.ok()
    }
}
