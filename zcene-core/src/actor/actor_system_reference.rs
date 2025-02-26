use crate::actor::{ActorAllocatorHandler, ActorSystem};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorSystemReference<H> = Arc<ActorSystem<H>, <H as ActorAllocatorHandler>::Allocator>;
