use crate::actor::{
    Actor, ActorAddressReference, ActorAllocatorHandler, ActorBoxFuture, ActorCommonBounds,
    ActorEnvironment, ActorMessage, ActorMessageSender, ActorSendError,
};
use alloc::boxed::Box;
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorMailboxMessageSender<M, E>: ActorCommonBounds
where
    M: ActorMessage,
    E: ActorEnvironment + ActorAllocatorHandler,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, E>;
}

impl<A, M, E, F> ActorMailboxMessageSender<M, E>
    for (ActorAddressReference<A, E>, PhantomData<(M, A)>, F)
where
    A: Actor<E>,
    E: ActorEnvironment + ActorAllocatorHandler,
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
