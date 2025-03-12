use crate::kernel::memory::KernelHeapMemoryAllocator;
use crate::memory::allocator::EmptyHeapMemoryAllocator;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;

////////////////////////////////////////////////////////////////////////////////////////////////////

// TODO: Remove

#[global_allocator]
pub static KERNEL_GLOBAL_HEAP_MEMORY_ALLOCATOR: KernelGlobalHeapMemoryAllocator =
    KernelGlobalHeapMemoryAllocator::uninitialized();

pub struct KernelGlobalHeapMemoryAllocator(
    SyncUnsafeCell<MaybeUninit<Arc<KernelHeapMemoryAllocator, EmptyHeapMemoryAllocator>>>,
);

impl KernelGlobalHeapMemoryAllocator {
    pub const fn uninitialized() -> Self {
        Self(SyncUnsafeCell::new(MaybeUninit::uninit()))
    }

    pub fn initialize(&self, allocator: Arc<KernelHeapMemoryAllocator, EmptyHeapMemoryAllocator>) {
        *unsafe { self.0.get().as_mut() }.unwrap() = MaybeUninit::new(allocator);
    }
}

unsafe impl GlobalAlloc for KernelGlobalHeapMemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { unsafe { self.0.get().as_mut().unwrap().assume_init_ref() }.alloc(layout) }
    }

    unsafe fn dealloc(&self, data: *mut u8, layout: Layout) {
        unsafe { unsafe { self.0.get().as_mut().unwrap().assume_init_ref() }.dealloc(data, layout) }
    }
}
