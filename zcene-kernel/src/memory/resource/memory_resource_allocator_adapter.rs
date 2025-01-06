use crate::memory::resource::MemoryResourceStrategy;
use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct MemoryResourceAllocatorAdapter<S>
where
    S: MemoryResourceStrategy,
{
    strategy: S,
}

unsafe impl<S> Allocator for MemoryResourceAllocatorAdapter<S>
where
    S: MemoryResourceStrategy,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}
