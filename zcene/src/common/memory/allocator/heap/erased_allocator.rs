use crate::common::memory::allocator::EmptyHeapMemoryAllocator;
use alloc::sync::Arc;
use core::alloc::{AllocError, Allocator, AllocatorClone, Layout, StaticAllocator};
use core::fmt::{self, Debug, Formatter};
use core::ptr::NonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct ErasedAllocator(Arc<dyn Allocator + Send + Sync, EmptyHeapMemoryAllocator>);

impl ErasedAllocator {
    pub fn new(allocator: Arc<dyn Allocator + Send + Sync, EmptyHeapMemoryAllocator>) -> Self {
        Self(allocator)
    }
}

impl Debug for ErasedAllocator {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ErasedAllocator")
            .finish_non_exhaustive()
    }
}

unsafe impl Allocator for ErasedAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.0.deallocate(ptr, layout) }
    }
}

// The erased allocator only ever forwards to the heap allocator it wraps,
// which never invalidates allocations except on explicit deallocation.
unsafe impl AllocatorClone for ErasedAllocator {}
unsafe impl StaticAllocator for ErasedAllocator {}
