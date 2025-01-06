use crate::{ActorCommonBounds, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorContextMessageProvider<M>: ActorCommonBounds
where
    M: ActorMessage,
{
    fn message(&self) -> &M;
}
