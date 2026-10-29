use crate::actor::{
    ActorEnvironment, ActorEnvironmentAllocator, ActorRecipient, ActorRecipientMessageSender,
    ActorMessage,
};
use alloc::sync::Weak;
use core::fmt::{self, Debug, Formatter};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorWeakRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    caller: Weak<dyn ActorRecipientMessageSender<M, E> + Send + Sync, E::Allocator>,
}

impl<M, E> Debug for ActorWeakRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.debug_struct("ActorWeakRecipient").finish()
    }
}

impl<M, E> ActorWeakRecipient<M, E>
where
    M: ActorMessage,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn upgrade(&self) -> Option<ActorRecipient<M, E>> {
        self.caller.upgrade().map(ActorRecipient::new)
    }
}
