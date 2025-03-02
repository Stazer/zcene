pub use crate::actor::{Actor, ActorAddress, ActorCommonBounds, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironment: ActorCommonBounds + Sized {
    type Address<A>: ActorAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext: ActorCommonBounds;
    type DestroyContext: ActorCommonBounds;
    type HandleContext<M>: ActorCommonBounds
    where
        M: ActorMessage;
}
