use crate::actor::{ActorAllocatorHandler, ActorHandler, ActorMailbox};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum RootActorMessage<H>
where
    H: ActorHandler + ActorAllocatorHandler,
{
    Attach(ActorMailbox<(), H>),
}

impl<H> Clone for RootActorMessage<H>
where
    H: ActorHandler + ActorAllocatorHandler,
{
    fn clone(&self) -> Self {
        match self {
            Self::Attach(mailbox) => Self::Attach(mailbox.clone()),
        }
    }
}
