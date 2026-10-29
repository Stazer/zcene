use crate::actor::{
    Actor, ActorAddress, ActorDyn, ActorDynCell, ActorEnvironment, ActorEnvironmentAllocator,
    ActorFuture, ActorMessageChannelSender, ActorMessageEnvelope, ActorMessageSender,
    ActorSendError,
};
use alloc::boxed::Box;
use core::marker::PhantomData;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    sender: ActorMessageChannelSender<
        ActorMessageEnvelope<A::Message, E>,
        <E as ActorEnvironmentAllocator>::Allocator,
    >,
    allocator: <E as ActorEnvironmentAllocator>::Allocator,
    #[Constructor(default)]
    handler_type: PhantomData<E>,
}

impl<A, E> Clone for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn clone(&self) -> Self {
        Self::new(self.sender.clone(), self.allocator.clone())
    }
}

impl<A, E> ActorMessageSender<A::Message> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.sender.transmit(ActorMessageEnvelope::Message(message)).await }
    }
}

impl<A, E> ActorAddress<A, E> for ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
}

impl<A, E> ActorMessageChannelAddress<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    /// Swap the actor instance behind this address for a different implementation that
    /// handles the same message type. The old instance is `destroy`ed and the new one
    /// `create`d by whichever task is running the executor loop for this recipient, in
    /// order relative to every other message already queued - no messages are lost and
    /// every clone of this address/recipient keeps working unchanged.
    pub fn migrate<A2>(&self, actor: A2) -> impl ActorFuture<'_, Result<(), ActorSendError>>
    where
        A2: Actor<E, Message = A::Message>,
    {
        let allocator = self.allocator.clone();

        async move {
            let cell: Box<dyn ActorDyn<E, Message = A::Message>, E::Allocator> =
                Box::new_in(ActorDynCell::new(actor, allocator.clone()), allocator);

            self.sender.transmit(ActorMessageEnvelope::Migrate(cell)).await
        }
    }
}
