use alloc::sync::Arc;
use crate::actor::{ActorEnvironmentAllocator};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorEnvironmentReference<E> = Arc<E, <E as ActorEnvironmentAllocator>::Allocator>;
