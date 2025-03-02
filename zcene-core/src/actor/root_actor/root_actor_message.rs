use crate::actor::{ActorAllocatorHandler, ActorEnvironment, ActorMailbox};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum RootActorMessage<E>
where
    E: ActorEnvironment + ActorAllocatorHandler,
{
    Attach(ActorMailbox<(), E>),
}

impl<E> Clone for RootActorMessage<E>
where
    E: ActorEnvironment + ActorAllocatorHandler,
{
    fn clone(&self) -> Self {
        match self {
            Self::Attach(mailbox) => Self::Attach(mailbox.clone()),
        }
    }
}
