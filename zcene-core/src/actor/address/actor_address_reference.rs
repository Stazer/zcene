use crate::actor::{ActorAllocatorHandler, ActorEnvironment};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorAddressReference<A, H> =
    Arc<<H as ActorEnvironment>::Address<A>, <H as ActorAllocatorHandler>::Allocator>;
