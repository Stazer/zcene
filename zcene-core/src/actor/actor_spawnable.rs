pub use crate::actor::{Actor, ActorCommonBounds, ActorEnvironment, ActorSpawnError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorSpawnable<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    fn spawn(self, environment: &E) -> Result<E::Address<A>, ActorSpawnError>;
}
