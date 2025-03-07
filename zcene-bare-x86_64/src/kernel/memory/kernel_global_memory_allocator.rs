use core::alloc::{GlobalAlloc, Layout};
use core::mem::MaybeUninit;
use zcene_bare::memory::allocator::EmptyMemoryAllocator;
use core::cell::SyncUnsafeCell;
use alloc::sync::Arc;
use crate::kernel::memory::KernelMemoryAllocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[global_allocator]
pub static KERNEL_GLOBAL_MEMORY_ALLOCATOR: KernelGlobalMemoryAllocator =
    KernelGlobalMemoryAllocator::uninitialized();

pub struct KernelGlobalMemoryAllocator(
    SyncUnsafeCell<MaybeUninit<Arc<KernelMemoryAllocator, EmptyMemoryAllocator>>>,
);

impl KernelGlobalMemoryAllocator {
    pub const fn uninitialized() -> Self {
        Self(SyncUnsafeCell::new(MaybeUninit::uninit()))
    }

    pub fn initialize(&self, allocator: Arc<KernelMemoryAllocator, EmptyMemoryAllocator>) {
        *unsafe { self.0.get().as_mut() }.unwrap() = MaybeUninit::new(allocator);
    }
}

unsafe impl GlobalAlloc for KernelGlobalMemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { self.0.get().as_mut().unwrap().assume_init_ref() }.alloc(layout)
    }

    unsafe fn dealloc(&self, data: *mut u8, layout: Layout) {
        unsafe { self.0.get().as_mut().unwrap().assume_init_ref() }.dealloc(data, layout)
    }
}
