use crate::actor::ActorEnvironmentAllocator;
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorEnvironmentReference<E> = Arc<E, <E as ActorEnvironmentAllocator>::Allocator>;
