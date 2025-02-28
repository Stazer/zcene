pub use crate::actor::{Actor, ActorCommonBounds, ActorHandler, ActorSpawnError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorSpawnHandler<H>
where
    H: ActorHandler,
{
    type SpawnSpecification<A>: ActorCommonBounds
    where
        A: ActorCommonBounds + Actor<H>;

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<H::Address<A>, ActorSpawnError>
    where
        A: Actor<H>;
}
