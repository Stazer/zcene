use crate::actor::{Actor, ActorAddressReference, ActorHandler};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorRelocationHandler: ActorHandler {
    fn relocate<A, B>(
        &self,
        first: ActorAddressReference<A, Self>,
        second: ActorAddressReference<B, Self>,
    ) where
        A: Actor<Self>,
        B: Actor<Self>;
}
