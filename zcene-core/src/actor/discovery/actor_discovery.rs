use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator, ActorRecipient, ActorMessage};
use alloc::vec::Vec;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn register<M>(&self, recipient: ActorRecipient<M, E>)
    where
        M: ActorMessage;

    fn lookup<M>(&self) -> Vec<ActorRecipient<M, E>, E::Allocator>
    where
        M: ActorMessage;
}
