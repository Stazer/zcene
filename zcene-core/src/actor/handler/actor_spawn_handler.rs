pub use crate::actor::{Actor, ActorCommonBounds, ActorHandler, ActorSpawnError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorSpawnHandler: ActorHandler
{
    type SpawnSpecification<A>: ActorCommonBounds
    where
        A: ActorCommonBounds + Actor<Self>;

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<Self::Address<A>, ActorSpawnError>
    where
        A: Actor<Self>;
}
