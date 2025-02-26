use crate::actor::{ActorHandler, ActorAllocatorHandler};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorAddressReference<A, H> =
    Arc<<H as ActorHandler>::Address<A>, <H as ActorAllocatorHandler>::Allocator>;
