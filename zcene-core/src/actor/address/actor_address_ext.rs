use crate::actor::{
    Actor, ActorAddress, ActorEnvironmentAllocator, ActorCommonBounds, ActorEnvironment, ActorMailbox,
    ActorMessage,
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
    fn mailbox_with_mapping<F, M>(
        self: &Arc<Self, E::Allocator>,
        mapping: F,
    ) -> Result<ActorMailbox<M, E>, AllocError>
    where
        F: Fn(M) -> A::Message + ActorCommonBounds,
        M: ActorMessage;

    fn mailbox(self: &Arc<Self, E::Allocator>) -> Result<ActorMailbox<A::Message, E>, AllocError> {
        self.mailbox_with_mapping(|m| m)
    }
}

impl<A, H, T> ActorAddressExt<A, H> for T
where
    A: Actor<H>,
    H: ActorEnvironment<Address<A> = T> + ActorEnvironmentAllocator,
    T: ActorAddress<A, H> + ActorCommonBounds,
{
    fn mailbox_with_mapping<F, M>(
        self: &Arc<Self, H::Allocator>,
        mapping: F,
    ) -> Result<ActorMailbox<M, H>, AllocError>
    where
        F: Fn(M) -> A::Message + ActorCommonBounds,
        M: ActorMessage,
    {
        let allocator = Arc::allocator(self).clone();

        Ok(ActorMailbox::new(Arc::try_new_in(
            (self.clone(), PhantomData::<(M, A)>, mapping),
            allocator,
        )?))
    }
}
