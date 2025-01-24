use crate::actor::{
    Actor, ActorAddress, ActorFuture, ActorHandler, ActorMessageChannelSender, ActorMessageSender,
    ActorSendError,
};
use core::marker::PhantomData;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorMessageChannelAddress<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    sender: ActorMessageChannelSender<A::Message>,
    handler_type: PhantomData<H>,
}

impl<A, H> ActorMessageSender<A::Message> for ActorMessageChannelAddress<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.sender.send(message).await }
    }
}

impl<A, H> ActorAddress<A, H> for ActorMessageChannelAddress<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
}
