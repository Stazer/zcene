use alloc::sync::Arc;
use crate::actor::{ActorEnvironment, ActorEnvironmentAllocator};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorEnvironmentReference<E> = Arc<E, <E as ActorEnvironmentAllocator>::Allocator>;
