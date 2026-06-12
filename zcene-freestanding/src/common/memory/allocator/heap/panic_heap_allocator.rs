use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use core::ptr::NonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct PanicHeapAllocator;

unsafe impl GlobalAlloc for PanicHeapAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        panic!("global allocator called")
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("global allocator called")
    }
}

unsafe impl Allocator for PanicHeapAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        panic!("global allocator called")
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        panic!("global allocator called")
    }
}
