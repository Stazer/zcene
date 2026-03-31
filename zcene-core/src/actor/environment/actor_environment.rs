pub use crate::actor::{Actor, ActorContext, ActorAddress, ActorCommonBounds, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironment: ActorCommonBounds + Sized {
    type Address<A>: ActorAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext<'a>: ActorContext;
    type HandleContext<'a, M>: ActorContext
    where
        M: ActorMessage;
    type DestroyContext<'a>: ActorContext;
}
