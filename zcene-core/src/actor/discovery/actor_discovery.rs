use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator, ActorMailbox, ActorMessage};
use alloc::vec::Vec;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn register<M>(&self, mailbox: ActorMailbox<M, E>)
    where
        M: ActorMessage;

    fn lookup<M>(&self) -> Vec<ActorMailbox<M, E>, E::Allocator>
    where
        M: ActorMessage;
}
