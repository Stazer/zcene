use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator, ActorMailbox};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum RootActorMessage<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    Attach(ActorMailbox<(), E>),
}

impl<E> Clone for RootActorMessage<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn clone(&self) -> Self {
        match self {
            Self::Attach(mailbox) => Self::Attach(mailbox.clone()),
        }
    }
}
