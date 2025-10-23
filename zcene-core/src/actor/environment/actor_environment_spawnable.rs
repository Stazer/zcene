pub use crate::actor::{
    Actor, ActorEnvironmentAllocator, ActorEnvironmentReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentSpawnable<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    type Address = E::Address<A>;

    fn spawn(self, system: &ActorEnvironmentReference<E>) -> Result<Self::Address, ActorSpawnError>;
}

use crate::actor::{ActorSpawnError, ActorEnvironment, };

pub trait ActorEnvironmentSpawn<A>
where
    A: Actor<Self>,
    Self: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn spawn(self: &ActorEnvironmentReference<Self>, actor: A) -> Result<Self::Address<A>, ActorSpawnError>;
}
