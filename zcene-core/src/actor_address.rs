use crate::{Actor, ActorCommonBounds, ActorHandler, ActorMessageSender};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAddress<A, H>: ActorMessageSender<A::Message> + ActorCommonBounds
where
    A: Actor<H>,
    H: ActorHandler,
{
}
