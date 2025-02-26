use crate::actor::{Actor, ActorAddressReference, ActorAllocatorHandler, ActorHandler};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorRelocateHandler: ActorHandler + ActorAllocatorHandler {
    fn relocate<A, B>(
        &self,
        first: ActorAddressReference<A, Self>,
        second: ActorAddressReference<B, Self>,
    ) where
        A: Actor<Self>,
        B: Actor<Self>;
}
