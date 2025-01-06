use crate::FutureRuntimeTask;
use alloc::alloc::Global;
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

// FIXME: Global allocator is required since ArcWake does not use the allocator_api feature
pub type FutureRuntimeTaskReference<H> = Arc<FutureRuntimeTask<H>, Global>;
