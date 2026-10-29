use crate::actor::{
    Actor, ActorAddress, ActorCommonBounds, ActorEnvironment, ActorEnvironmentAllocator,
    ActorRecipient, ActorMessage,
};
use alloc::sync::Arc;
use core::alloc::AllocError;
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAddressExt<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn recipient_with_mapping<F, M>(
        self: &Arc<Self, E::Allocator>,
        mapping: F,
    ) -> Result<ActorRecipient<M, E>, AllocError>
    where
        F: Fn(M) -> A::Message + ActorCommonBounds,
        M: ActorMessage;

    fn recipient(self: &Arc<Self, E::Allocator>) -> Result<ActorRecipient<A::Message, E>, AllocError> {
        self.recipient_with_mapping(|m| m)
    }
}

impl<A, H, T> ActorAddressExt<A, H> for T
where
    A: Actor<H>,
    H: ActorEnvironment<Address<A> = T> + ActorEnvironmentAllocator,
    T: ActorAddress<A, H> + ActorCommonBounds,
{
    fn recipient_with_mapping<F, M>(
        self: &Arc<Self, H::Allocator>,
        mapping: F,
    ) -> Result<ActorRecipient<M, H>, AllocError>
    where
        F: Fn(M) -> A::Message + ActorCommonBounds,
        M: ActorMessage,
    {
        let allocator = Arc::allocator(self).clone();

        Ok(ActorRecipient::new(Arc::try_new_in(
            (self.clone(), PhantomData::<(M, A)>, mapping),
            allocator,
        )?))
    }
}
