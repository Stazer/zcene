use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorAddressReference<A, E> =
    Arc<<E as ActorEnvironment>::Address<A>, <E as ActorEnvironmentAllocator>::Allocator>;
