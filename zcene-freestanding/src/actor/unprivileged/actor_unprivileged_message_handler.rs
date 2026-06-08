use alloc::boxed::Box;
use core::fmt::Debug;
use zcene_core::actor::{
    Actor, ActorBoxFuture, ActorCommonBounds, ActorEnvironment, ActorEnvironmentAllocator,
    ActorMessage, ActorMessageChannelAddress, ActorMessageSender, ActorSendError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorUnprivilegedMessageHandler<E>: ActorCommonBounds
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(
        &self,
        allocator: &E::Allocator,
        message: *const (),
    ) -> ActorBoxFuture<'static, Result<(), ActorSendError>, E>;
}

impl<A, E> ActorUnprivilegedMessageHandler<E> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    A::Message: Debug,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(
        &self,
        allocator: &E::Allocator,
        message: *const (),
    ) -> ActorBoxFuture<'static, Result<(), ActorSendError>, E> {
        let sender = self.clone();

        // SAFETY: caller guarantees message points to a valid A::Message
        let message = unsafe { message.cast::<A::Message>().as_ref().unwrap() }.clone();

        Box::pin_in(
            async move { <Self as ActorMessageSender<_>>::send(&sender, message).await },
            allocator.clone(),
        )
    }
}
