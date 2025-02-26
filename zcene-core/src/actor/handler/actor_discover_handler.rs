use crate::actor::{ActorAllocatorHandler, ActorHandler, ActorMailbox, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorDiscoverHandler: ActorHandler + ActorAllocatorHandler {
    fn discover<M>(&self) -> Option<ActorMailbox<M, Self>>
    where
        M: ActorMessage;
}
