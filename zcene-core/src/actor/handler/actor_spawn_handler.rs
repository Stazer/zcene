pub use crate::actor::{
    Actor, ActorAddressReference, ActorAllocatorHandler, ActorCommonBounds, ActorHandler,
    ActorSpawnError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorSpawnHandler: ActorHandler + ActorAllocatorHandler {
    type SpawnSpecification<A>: ActorCommonBounds
    where
        A: Actor<Self>;

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>;
}
