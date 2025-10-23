pub use crate::actor::{
    ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator, ActorEnvironmentReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentEnterable<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn enter(self, system: &ActorEnvironmentReference<E>) -> Result<(), ActorEnterError>;
}
