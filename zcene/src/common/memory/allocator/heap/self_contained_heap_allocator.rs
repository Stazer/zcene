use crate::common::memory::allocator::EmptyHeapMemoryAllocator;
use alloc::sync::Arc;
use core::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::NonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct SelfContainedHeapAllocator<A>
where
    A: Allocator,
{
    allocator: ManuallyDrop<A>,
}

impl<A> SelfContainedHeapAllocator<A>
where
    A: Allocator,
{
    pub fn new(allocator: A) -> Result<Arc<Self, EmptyHeapMemoryAllocator>, AllocError> {
        let uninitialized = Arc::<Self, &A>::try_new_uninit_in(&allocator)?;

        let (pointer, _) = Arc::into_raw_with_allocator(uninitialized);

        unsafe {
            pointer.cast_mut().write(MaybeUninit::new(Self {
                allocator: ManuallyDrop::new(allocator),
            }))
        };

        Ok(unsafe { Arc::from_raw_in(pointer.cast::<Self>(), EmptyHeapMemoryAllocator) })
    }
}

unsafe impl<A> Allocator for SelfContainedHeapAllocator<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.allocator.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.allocator.deallocate(ptr, layout) }
    }
}

unsafe impl<A> GlobalAlloc for SelfContainedHeapAllocator<A>
where
    A: Allocator + GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { self.allocator.alloc(layout) }
    }

    unsafe fn dealloc(&self, data: *mut u8, layout: Layout) {
        unsafe { self.allocator.dealloc(data, layout) }
    }
}
