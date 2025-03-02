use crate::actor::{ActorSystem, ActorEnvironmentAllocator};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorSystemReference<E> = Arc<ActorSystem<E>, <E as ActorEnvironmentAllocator>::Allocator>;
