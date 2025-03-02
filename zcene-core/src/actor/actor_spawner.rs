pub use crate::actor::{Actor, ActorEnvironment, ActorSpawnError, ActorSpawnable};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorSpawner<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    fn spawn<S>(&self, spawnable: S) -> Result<E::Address<A>, ActorSpawnError>
    where
        S: ActorSpawnable<A, E>;
}
