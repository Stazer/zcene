use crate::actor::{Actor, ActorAllocatorHandler, ActorAddressReference, ActorHandler};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorRelocationHandler: ActorHandler + ActorAllocatorHandler {
    fn relocate<A, B>(
        &self,
        first: ActorAddressReference<A, Self>,
        second: ActorAddressReference<B, Self>,
    ) where
        A: Actor<Self>,
        B: Actor<Self>;
}
