use crate::actor::{
    Actor, ActorAddressReference, ActorBoxFuture, ActorCommonBounds, ActorEnvironment,
    ActorEnvironmentAllocator, ActorMessage, ActorMessageSender, ActorSendError,
};
use alloc::boxed::Box;
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorRecipientMessageSender<M, E>: Send + Sync + 'static
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, E>;
}

impl<A, M, E, F> ActorRecipientMessageSender<M, E>
    for (ActorAddressReference<A, E>, PhantomData<(M, A)>, F)
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
    M: ActorMessage,
    F: Fn(M) -> A::Message + ActorCommonBounds,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, E> {
        let allocator = ActorAddressReference::<A, E>::allocator(&self.0).clone();
        let sender = self.0.clone();
        let message = (self.2)(message);

        Box::pin_in(async move { sender.send(message).await }, allocator)
    }
}
