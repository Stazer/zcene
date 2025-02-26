use crate::actor::{
    Actor, ActorAddressReference, ActorAllocatorHandler, ActorBoxFuture, ActorCommonBounds,
    ActorHandler, ActorMessage, ActorMessageSender, ActorSendError,
};
use alloc::boxed::Box;
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorMailboxMessageSender<M, H>: ActorCommonBounds
where
    M: ActorMessage,
    H: ActorHandler + ActorAllocatorHandler,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, H>;
}

impl<A, M, H, F> ActorMailboxMessageSender<M, H>
    for (ActorAddressReference<A, H>, PhantomData<(M, A)>, F)
where
    A: Actor<H>,
    H: ActorHandler + ActorAllocatorHandler,
    M: ActorMessage,
    F: Fn(M) -> A::Message + ActorCommonBounds,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, H> {
        let allocator = ActorAddressReference::<A, H>::allocator(&self.0).clone();
        let sender = self.0.clone();
        let message = (self.2)(message);

        Box::pin_in(async move { sender.send(message).await }, allocator)
    }
}
