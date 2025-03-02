pub use crate::actor::{ActorEnvironment, ActorEnterError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnterable<E>
where
    E: ActorEnvironment,
{
    fn enter(self, environment: &E) -> Result<(), ActorEnterError>;
}
