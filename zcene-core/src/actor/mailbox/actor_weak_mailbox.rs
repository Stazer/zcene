use crate::actor::{
    ActorEnvironment, ActorEnvironmentAllocator, ActorMailbox, ActorMailboxMessageSender,
    ActorMessage,
};
use alloc::sync::Weak;
use core::fmt::{self, Debug, Formatter};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorWeakMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    caller: Weak<dyn ActorMailboxMessageSender<M, E>, E::Allocator>,
}

impl<M, E> Debug for ActorWeakMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorWeakMailbox").finish()
    }
}

impl<M, E> ActorWeakMailbox<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn upgrade(&self) -> Option<ActorMailbox<M, E>> {
        self.caller.upgrade().map(ActorMailbox::new)
    }
}
