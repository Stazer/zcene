pub use crate::actor::{
    ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator, ActorSystemReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentEnterable<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn enter(self, system: &ActorSystemReference<E>) -> Result<(), ActorEnterError>;
}
