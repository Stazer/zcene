pub use crate::actor::{Actor, ActorCommonBounds, ActorHandler, ActorSpawnError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorSpawnHandler<T>: ActorHandler
where
    T: ActorHandler,
{
    type SpawnSpecification<A>: ActorCommonBounds
    where
        A: ActorCommonBounds + Actor<T>;

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<Self::Address<A>, ActorSpawnError>
    where
        A: Actor<Self> + Actor<T>;
}
