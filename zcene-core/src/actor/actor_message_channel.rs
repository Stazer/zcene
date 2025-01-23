use crate::actor::{ActorMessage, ActorMessageChannelReceiver, ActorMessageChannelSender};
use async_channel::{bounded, unbounded};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorMessageChannel<M>(PhantomData<M>)
where
    M: ActorMessage;

impl<M> ActorMessageChannel<M>
where
    M: ActorMessage,
{
    pub fn new_unbounded() -> (ActorMessageChannelSender<M>, ActorMessageChannelReceiver<M>) {
        let (sender, receiver) = bounded(100);

        (
            ActorMessageChannelSender::new(sender),
            ActorMessageChannelReceiver::new(receiver),
        )
    }
}
