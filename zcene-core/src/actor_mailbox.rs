use crate::{
    Actor, ActorAddressReference, ActorBoxFuture, ActorCommonBounds, ActorFuture,
    ActorHandler, ActorMessage, ActorMessageSender, ActorSendError,
};
use alloc::boxed::Box;
use core::alloc::AllocError;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorMailboxMessageSender<M, H>: ActorCommonBounds
where
    M: ActorMessage,
    H: ActorHandler,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, H>;
}

impl<A, M, H> ActorMailboxMessageSender<M, H>
    for (
        ActorAddressReference<A, H>,
        core::marker::PhantomData<(M, A)>,
    )
where
    A: Actor<H>,
    H: ActorHandler,
    M: ActorMessage,
    A::Message: From<M>,
{
    fn send(&self, message: M) -> ActorBoxFuture<'_, Result<(), ActorSendError>, H> {
        let allocator = ActorAddressReference::<A, H>::allocator(&self.0).clone();

        let sender = self.0.clone();

        Box::pin_in(async move { sender.send(message.into()).await }, allocator)
    }
}

pub struct ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    caller: Arc<dyn ActorMailboxMessageSender<M, H>, H::Allocator>,
}

impl<M, H> Clone for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    fn clone(&self) -> Self {
        Self {
            caller: self.caller.clone(),
        }
    }
}

impl<M, H> ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    pub fn try_new_from_address<A>(address: ActorAddressReference<A, H>) -> Result<Self, AllocError>
    where
        A: Actor<H, Message = M>,
        H::Allocator: 'static,
    {
        let allocator = ActorAddressReference::<A, H>::allocator(&address).clone();

        Ok(Self {
            caller: Arc::try_new_in((address, core::marker::PhantomData::<(M, A)>), allocator)?,
        })
    }
}

impl<M, H> ActorMessageSender<M> for ActorMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async { self.caller.send(message).await }
    }
}

use alloc::sync::Arc;

pub trait ActorAddressExt<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn mailbox(self: Arc<Self>) -> ActorMailbox<A::Message, H>;
}
