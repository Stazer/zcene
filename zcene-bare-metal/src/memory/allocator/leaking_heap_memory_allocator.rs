use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use core::ptr::{NonNull, null_mut};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct LeakingHeapMemoryAllocator;

unsafe impl GlobalAlloc for LeakingHeapMemoryAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _data: *mut u8, _layout: Layout) {}
}

unsafe impl Allocator for LeakingHeapMemoryAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _data: NonNull<u8>, _layout: Layout) {}
}
