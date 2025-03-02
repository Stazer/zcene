pub use crate::actor::{ActorEnterError, ActorEnvironment};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentEnterable<E>
where
    E: ActorEnvironment,
{
    fn enter(self, environment: &E) -> Result<(), ActorEnterError>;
}
