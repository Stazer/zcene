use crate::actor::{ActorHandler, ActorMailbox};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum RootActorMessage<H>
where
    H: ActorHandler
{
    Attach(ActorMailbox<(), H>),
}

impl<H> Clone for RootActorMessage<H>
where
    H: ActorHandler
{
    fn clone(&self) -> Self {
        match self {
            Self::Attach(mailbox) => Self::Attach(mailbox.clone())
        }
    }
}
