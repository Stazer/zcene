use crate::actor::{
    ActorAllocator, ActorCommonBounds, ActorMessageChannelReceiver, ActorMessageChannelSender,
};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorMessageChannel<M>(PhantomData<M>)
where
    M: ActorCommonBounds;

impl<M> ActorMessageChannel<M>
where
    M: ActorCommonBounds,
{
    pub fn new_unbounded_in<A>(
        allocator: A,
    ) -> (ActorMessageChannelSender<M, A>, ActorMessageChannelReceiver<M, A>)
    where
        A: ActorAllocator,
    {
        let (sender, receiver) = crate::channel::mpsc::unbounded_in(allocator);

        (
            ActorMessageChannelSender::new(sender),
            ActorMessageChannelReceiver::new(receiver),
        )
    }
}
