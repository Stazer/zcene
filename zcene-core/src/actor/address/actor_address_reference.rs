use crate::actor::{ActorAllocatorHandler, ActorHandler};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorAddressReference<A, H> =
    Arc<<H as ActorHandler>::Address<A>, <H as ActorAllocatorHandler>::Allocator>;
