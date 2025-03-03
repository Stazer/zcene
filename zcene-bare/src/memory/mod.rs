pub mod frame;
//pub mod resource;
pub mod address;
pub mod region;

pub struct LeakingAllocator;

pub type LeakingBox<T> = Box<T, LeakingAllocator>;

use alloc::boxed::Box;
use core::alloc::AllocError;
use core::alloc::Allocator;
use core::alloc::Layout;
use core::ptr::NonNull;

unsafe impl Allocator for LeakingAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _data: NonNull<u8>, _layout: Layout) {}
}
