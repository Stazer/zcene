use core::alloc::{Allocator, AllocError, GlobalAlloc, Layout};
use core::ptr::{null_mut, NonNull};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct LeakingMemoryAllocator;

unsafe impl GlobalAlloc for LeakingMemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {}
}

unsafe impl Allocator for LeakingMemoryAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, mut data: NonNull<u8>, layout: Layout) {}
}
