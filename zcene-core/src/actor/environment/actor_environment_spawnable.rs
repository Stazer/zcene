pub use crate::actor::{Actor, ActorEnvironment, ActorSpawnError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentSpawnable<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    fn spawn(self, environment: &E) -> Result<E::Address<A>, ActorSpawnError>;
}
