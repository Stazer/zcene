use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator, ActorRecipient};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum RootActorMessage<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    Attach(ActorRecipient<(), E>),
}

impl<E> Clone for RootActorMessage<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn clone(&self) -> Self {
        match self {
            Self::Attach(recipient) => Self::Attach(recipient.clone()),
        }
    }
}
