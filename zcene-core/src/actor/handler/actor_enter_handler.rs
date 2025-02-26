pub use crate::actor::{ActorCommonBounds, ActorEnterError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnterHandler {
    type EnterSpecification: ActorCommonBounds;

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError>;
}
