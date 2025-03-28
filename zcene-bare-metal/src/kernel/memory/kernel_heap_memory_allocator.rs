use core::alloc::Allocator;
use core::alloc::{AllocError, GlobalAlloc, Layout};
use core::ptr::NonNull;
use linked_list_allocator::LockedHeap;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct KernelHeapMemoryAllocator(LockedHeap);

unsafe impl Allocator for KernelHeapMemoryAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        NonNull::new(unsafe { self.0.alloc(layout) })
            .map(|pointer| unsafe { NonNull::slice_from_raw_parts(pointer, layout.size()) })
            .ok_or(AllocError)
    }

    unsafe fn deallocate(&self, mut data: NonNull<u8>, layout: Layout) {
        unsafe { self.dealloc(data.as_mut(), layout) }
    }
}

unsafe impl GlobalAlloc for KernelHeapMemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { self.0.alloc(layout) }
    }

    unsafe fn dealloc(&self, data: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(data, layout) }
    }
}
