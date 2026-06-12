use crate::actor::{
    ActorAllocator, ActorMessage, ActorMessageChannelReceiver, ActorMessageChannelSender,
};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorMessageChannel<M>(PhantomData<M>)
where
    M: ActorMessage;

impl<M> ActorMessageChannel<M>
where
    M: ActorMessage,
{
    pub fn new_unbounded_in<A>(
        allocator: A,
    ) -> (ActorMessageChannelSender<M, A>, ActorMessageChannelReceiver<M, A>)
    where
        A: ActorAllocator,
    {
        let (sender, receiver) = futures_channel::mpsc::unbounded_in(allocator);

        (
            ActorMessageChannelSender::new(sender),
            ActorMessageChannelReceiver::new(receiver),
        )
    }
}
