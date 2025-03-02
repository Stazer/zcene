use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator, ActorMessage, ActorWeakMailbox};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorDiscoveryBucket<M, E> =
    Vec<ActorWeakMailbox<M, E>, <E as ActorEnvironmentAllocator>::Allocator>;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorMailboxDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    bucket_resolvers: BTreeMap<TypeId, Box<dyn Any + Send>>,
    types: PhantomData<E>,
}

impl<E> ActorMailboxDiscovery<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn bucket<M>(&self) -> Option<&ActorDiscoveryBucket<M, E>>
    where
        M: ActorMessage,
    {
        let bucket_resolver = match self.bucket_resolvers.get(&TypeId::of::<M>()) {
            Some(bucket) => bucket,
            None => return None,
        };

        bucket_resolver.downcast_ref::<ActorDiscoveryBucket<M, E>>()
    }

    pub fn bucket_mut<M>(&mut self) -> Option<&mut ActorDiscoveryBucket<M, E>>
    where
        M: ActorMessage,
    {
        let bucket_resolver = match self.bucket_resolvers.get_mut(&TypeId::of::<M>()) {
            Some(bucket) => bucket,
            None => return None,
        };

        bucket_resolver.downcast_mut::<ActorDiscoveryBucket<M, E>>()
    }

    pub fn lookup<M>(&self) -> impl Iterator<Item = &ActorWeakMailbox<M, E>>
    where
        M: ActorMessage,
    {
        self.bucket::<M>()
            .map(|data| data.iter())
            .into_iter()
            .flatten()
    }
}
