use crate::actor::{
    Actor, ActorAddress, ActorEnvironment, ActorFuture, ActorMessageChannelSender,
    ActorMessageSender, ActorSendError,
};
use core::marker::PhantomData;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    sender: ActorMessageChannelSender<A::Message>,
    #[Constructor(default)]
    handler_type: PhantomData<E>,
}

impl<A, E> Clone for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    fn clone(&self) -> Self {
        Self::new(self.sender.clone())
    }
}

impl<A, E> ActorMessageSender<A::Message> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.sender.send(message).await }
    }
}

impl<A, E> ActorAddress<A, E> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
}
