pub use crate::actor::{
    Actor, ActorAddress, ActorAddressReference, ActorAllocator, ActorCommonBounds, ActorEnterError,
    ActorMessage, ActorSpawnError,
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

    fn allocator(&self) -> &Self::Allocator;

    fn spawn<A>(&self, specification: Self::SpawnSpecification<A>) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>;

    fn enter(&self) -> Result<(), ActorEnterError>;
}
