use crate::actor::{Actor, ActorCommonBounds, ActorEnvironment, ActorMessageSender};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAddress<A, E>: ActorMessageSender<A::Message> + ActorCommonBounds
where
    A: Actor<E>,
    E: ActorEnvironment,
{
}
