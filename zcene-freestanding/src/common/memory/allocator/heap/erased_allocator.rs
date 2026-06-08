use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct ErasedAllocator(&'static (dyn Allocator + Send + Sync));

impl ErasedAllocator {
    pub fn new(allocator: &'static (dyn Allocator + Send + Sync)) -> Self {
        Self(allocator)
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
