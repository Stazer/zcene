pub use crate::actor::{Actor, ActorEnvironmentAllocator, ActorEnvironmentReference};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentSpawnable<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    type Address = E::Address<A>;

    fn spawn(self, system: &ActorEnvironmentReference<E>)
        -> Result<Self::Address, ActorSpawnError>;
}

use crate::actor::{ActorEnvironment, ActorSpawnError};
