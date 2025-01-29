use crate::actor::{ActorHandler, ActorMailbox, ActorMailboxMessageSender, ActorMessage};
use alloc::sync::Weak;
use core::fmt::{self, Debug, Formatter};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorWeakMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    caller: Weak<dyn ActorMailboxMessageSender<M, H>, H::Allocator>,
}

impl<M, H> Debug for ActorWeakMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorWeakMailbox").finish()
    }
}

impl<M, H> ActorWeakMailbox<M, H>
where
    M: ActorMessage,
    H: ActorHandler,
{
    pub fn upgrade(&self) -> Option<ActorMailbox<M, H>> {
        self.caller.upgrade().map(ActorMailbox::new)
    }
}
