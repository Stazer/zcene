use crate::actor::{ActorHandler, ActorMailbox, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorDiscoveryHandler: ActorHandler {
    fn discover<M>(&self) -> Option<ActorMailbox<M, Self>>
    where
        M: ActorMessage;
}
