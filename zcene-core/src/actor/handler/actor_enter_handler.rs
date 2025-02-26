pub use crate::actor::{
    ActorEnterError, ActorHandler, ActorCommonBounds,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnterHandler: ActorHandler {
    type EnterSpecification: ActorCommonBounds;

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError>;
}
