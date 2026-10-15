use crate::common::memory::allocator::{EmptyHeapMemoryAllocator, SelfContainedHeapAllocator};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type SelfContainedHeapAllocatorReference<A> =
    Arc<SelfContainedHeapAllocator<A>, EmptyHeapMemoryAllocator>;
