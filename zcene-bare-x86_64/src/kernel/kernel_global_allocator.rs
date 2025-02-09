use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use linked_list_allocator::LockedHeap;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[global_allocator]
pub static KERNEL_GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::empty();

pub struct GlobalAllocator(LockedHeap);

impl GlobalAllocator {
    pub const fn empty() -> Self {
        Self(LockedHeap::empty())
    }

    pub fn inner(&self) -> &LockedHeap {
        &self.0
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.dealloc(ptr, layout)
    }
}
