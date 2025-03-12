pub use crate::actor::{Actor, ActorAddress, ActorCommonBounds, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironment: ActorCommonBounds + Sized {
    type Address<A>: ActorAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext: ActorCommonBounds;
    type CreateContext2<'a>: Send + Sync = ();
    type HandleContext<M>: ActorCommonBounds
    where
        M: ActorMessage;
    type DestroyContext: ActorCommonBounds;
}
