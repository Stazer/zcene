use crate::actor::{
    Actor, ActorAddress, ActorFuture, ActorHandler, ActorMessageSender, ActorSendError,
};
use async_channel::Sender;
use core::marker::PhantomData;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[Constructor(visibility = pub(crate))]
pub struct FutureRuntimeActorAddress<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    sender: Sender<A::Message>,
    handler_type: PhantomData<H>,
}

impl<A, H> ActorMessageSender<A::Message> for FutureRuntimeActorAddress<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move {
            self.sender
                .send(message)
                .await
                .map_err(|_| ActorSendError::Closed)
        }
    }
}

impl<A, H> ActorAddress<A, H> for FutureRuntimeActorAddress<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
}
