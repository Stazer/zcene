use alloc::boxed::Box;
use core::fmt::Debug;
use zcene_core::actor::{
    Actor, ActorBoxFuture, ActorCommonBounds, ActorEnvironment, ActorEnvironmentAllocator,
    ActorMessage, ActorMessageChannelAddress, ActorMessageSender,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorIsolationMessageHandler<E>: ActorCommonBounds
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, allocator: &E::Allocator, message: *const ()) -> ActorBoxFuture<'static, (), E>;
}

impl<A, E> ActorIsolationMessageHandler<E> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    A::Message: Debug,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, allocator: &E::Allocator, message: *const ()) -> ActorBoxFuture<'static, (), E> {
        let sender = self.clone();

        // TODO: check address and length!
        let message = unsafe { message.cast::<A::Message>().as_ref().unwrap() }.clone();

        Box::pin_in(
            async move {
                <Self as ActorMessageSender<_>>::send(&sender, message).await;
            },
            allocator.clone(),
        )
    }
}
