pub use crate::actor::{
    Actor, ActorEnvironment, ActorEnvironmentAllocator, ActorSpawnError, ActorSystemReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentSpawnable<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    type Address = E::Address<A>;

    fn spawn(self, system: &ActorSystemReference<E>) -> Result<Self::Address, ActorSpawnError>;
}
