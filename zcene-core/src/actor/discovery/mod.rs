use crate::actor::{ActorAllocatorHandler, ActorEnvironment, ActorMessage, ActorWeakMailbox};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorDiscoveryBucket<M, H> =
    Vec<ActorWeakMailbox<M, H>, <H as ActorAllocatorHandler>::Allocator>;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorMailboxDiscovery<H>
where
    H: ActorEnvironment + ActorAllocatorHandler,
{
    bucket_resolvers: BTreeMap<TypeId, Box<dyn Any + Send>>,
    types: PhantomData<H>,
}

impl<H> ActorMailboxDiscovery<H>
where
    H: ActorEnvironment + ActorAllocatorHandler,
{
    pub fn bucket<M>(&self) -> Option<&ActorDiscoveryBucket<M, H>>
    where
        M: ActorMessage,
    {
        let bucket_resolver = match self.bucket_resolvers.get(&TypeId::of::<M>()) {
            Some(bucket) => bucket,
            None => return None,
        };

        bucket_resolver.downcast_ref::<ActorDiscoveryBucket<M, H>>()
    }

    pub fn bucket_mut<M>(&mut self) -> Option<&mut ActorDiscoveryBucket<M, H>>
    where
        M: ActorMessage,
    {
        let bucket_resolver = match self.bucket_resolvers.get_mut(&TypeId::of::<M>()) {
            Some(bucket) => bucket,
            None => return None,
        };

        bucket_resolver.downcast_mut::<ActorDiscoveryBucket<M, H>>()
    }

    pub fn lookup<M>(&self) -> impl Iterator<Item = &ActorWeakMailbox<M, H>>
    where
        M: ActorMessage,
    {
        self.bucket::<M>()
            .map(|data| data.iter())
            .into_iter()
            .flatten()
    }
}
