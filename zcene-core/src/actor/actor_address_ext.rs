use crate::actor::{Actor, ActorCommonBounds, ActorAddress, ActorHandler, ActorMailbox, ActorMessage};
use alloc::sync::Arc;
use core::alloc::AllocError;
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAddressExt<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn mailbox_with_mapping<F, M>(self: &Arc<Self, H::Allocator>, mapping: F) -> Result<ActorMailbox<M, H>, AllocError>
    where
        F: Fn(M) -> A::Message + ActorCommonBounds,
        M: ActorMessage;

    fn mailbox(self: &Arc<Self, H::Allocator>) -> Result<ActorMailbox<A::Message, H>, AllocError> {
        self.mailbox_with_mapping(|m| m)
    }
}

impl<A, H, T> ActorAddressExt<A, H> for T
where
    A: Actor<H>,
    H: ActorHandler<Address<A> = T>,
    T: ActorAddress<A, H> + ActorCommonBounds,
{
    fn mailbox_with_mapping<F, M>(self: &Arc<Self, H::Allocator>, mapping: F) -> Result<ActorMailbox<M, H>, AllocError>
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
