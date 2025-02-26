pub use crate::actor::{
    Actor, ActorAddress, ActorAllocator, ActorCommonBounds,
    ActorMessage,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorHandler: ActorCommonBounds + Sized {
    type Allocator: ActorAllocator;

    type Address<A>: ActorAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext: ActorCommonBounds;
    type DestroyContext: ActorCommonBounds;
    type HandleContext<M>: ActorCommonBounds
    where
        M: ActorMessage;

    type SpawnSpecification<A>: ActorCommonBounds
    where
        A: Actor<Self>;
    type EnterSpecification: ActorCommonBounds;

    fn allocator(&self) -> &Self::Allocator;
}
