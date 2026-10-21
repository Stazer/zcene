use crate::actor::{
    ActorAllocator, ActorCommonBounds, ActorFuture, ActorMessage, ActorMessageSender,
    ActorSendError,
};
use crate::channel::mpsc::UnboundedSender;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[Constructor(visibility = pub(super))]
pub struct ActorMessageChannelSender<M, A>
where
    M: ActorCommonBounds,
    A: ActorAllocator,
{
    sender: UnboundedSender<M, A>,
}

impl<M, A> Clone for ActorMessageChannelSender<M, A>
where
    M: ActorCommonBounds,
    A: ActorAllocator,
{
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone() }
    }
}

impl<M, A> ActorMessageChannelSender<M, A>
where
    M: ActorCommonBounds,
    A: ActorAllocator,
{
    /// Raw send that bypasses the `ActorMessageSender` trait, for payloads (like
    /// `ActorMessageEnvelope`) that carry non-`Clone` state and therefore aren't an
    /// `ActorMessage` themselves.
    pub(crate) fn transmit(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move {
            self.sender
                .unbounded_send(message)
                .map_err(|_| ActorSendError::Closed)
        }
    }
}

impl<M, A> ActorMessageSender<M> for ActorMessageChannelSender<M, A>
where
    M: ActorMessage,
    A: ActorAllocator,
{
    fn send(&self, message: M) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        self.transmit(message)
    }
}
