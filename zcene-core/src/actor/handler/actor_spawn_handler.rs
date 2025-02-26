pub use crate::actor::{
    ActorCommonBounds,
    Actor, ActorAddressReference, ActorSpawnError, ActorHandler, ActorAllocatorHandler,
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
