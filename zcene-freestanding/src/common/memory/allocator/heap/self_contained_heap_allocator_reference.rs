use crate::common::memory::allocator::{EmptyHeapMemoryAllocator, SelfContainedHeapAllocator};
use alloc::sync::Arc;
use core::alloc::Allocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type SelfContainedHeapAllocatorReference<A> = Arc<SelfContainedHeapAllocator<A>, EmptyHeapMemoryAllocator>
where
    A: Allocator;
