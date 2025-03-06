pub use crate::actor::{Actor, ActorEnvironment, ActorSpawnError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentSpawnable<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    type Address = E::Address<A>;

    fn spawn(self, environment: &E) -> Result<Self::Address, ActorSpawnError>;
}
