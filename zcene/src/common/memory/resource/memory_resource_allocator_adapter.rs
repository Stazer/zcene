use crate::common::synchronization::SpinMutex;
use crate::common::memory::resource::{
    MemoryResourceAllocationRequest, MemoryResourceDeallocationRequest, MemoryResourceStrategy,
};
use core::alloc::{AllocError, Allocator, Layout};
use core::num::NonZero;
use core::ptr::NonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////

struct MemoryResourceAllocatorAdapterInner<S> {
    memory: &'static mut [u8],
    strategy: S,
}

/// Turns a `MemoryResourceStrategy` plus the memory it manages into a `core::alloc::Allocator`,
/// e.g. an arena for one actor.
pub struct MemoryResourceAllocatorAdapter<S>
where
    S: MemoryResourceStrategy,
{
    inner: SpinMutex<MemoryResourceAllocatorAdapterInner<S>>,
}

impl<S> MemoryResourceAllocatorAdapter<S>
where
    S: MemoryResourceStrategy,
{
    pub fn new(memory: &'static mut [u8], mut strategy: S) -> Self {
        strategy.initialize(memory);

        Self {
            inner: SpinMutex::new(MemoryResourceAllocatorAdapterInner { memory, strategy }),
        }
    }
}

unsafe impl<S> Allocator for MemoryResourceAllocatorAdapter<S>
where
    S: MemoryResourceStrategy,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let Some(size) = NonZero::new(layout.size()) else {
            // Zero-sized: nothing to hand out, any aligned dangling pointer will do.
            return Ok(NonNull::slice_from_raw_parts(layout.dangling_ptr(), 0));
        };

        let mut inner = self.inner.lock();
        let inner = &mut *inner;

        let alignment = NonZero::new(layout.align()).ok_or(AllocError)?;

        let region = inner
            .strategy
            .allocate(inner.memory, MemoryResourceAllocationRequest::new(size, alignment))
            .map_err(|_error| AllocError)?;

        // SAFETY: the strategy only returns regions inside `memory`.
        let pointer = unsafe { inner.memory.as_mut_ptr().add(region.data_start()) };

        // Strategies align relative to the start of the memory, which only matches the real
        // alignment if the memory itself is aligned enough. Refuse rather than return bad pointers.
        if pointer as usize % layout.align() != 0 {
            return Err(AllocError);
        }

        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(pointer).ok_or(AllocError)?,
            size.get(),
        ))
    }

    unsafe fn deallocate(&self, pointer: NonNull<u8>, layout: Layout) {
        let (Some(size), Some(alignment)) =
            (NonZero::new(layout.size()), NonZero::new(layout.align()))
        else {
            return;
        };

        let mut inner = self.inner.lock();
        let inner = &mut *inner;

        let position = (pointer.as_ptr() as usize).wrapping_sub(inner.memory.as_ptr() as usize);

        let _result = inner.strategy.deallocate(
            inner.memory,
            MemoryResourceDeallocationRequest::new(position, size, alignment),
        );
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn allocator_with_bump_strategy() {
    use crate::common::memory::resource::MemoryResourceBumpStrategy;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    #[repr(align(16))]
    struct Memory([u8; 256]);

    let memory: &'static mut Memory = alloc::boxed::Box::leak(Box::new(Memory([0; 256])));
    let allocator = MemoryResourceAllocatorAdapter::new(&mut memory.0, MemoryResourceBumpStrategy::default());

    let start = memory_start(&allocator);

    let first = Box::new_in(1u64, &allocator);
    let second = Box::new_in(2u64, &allocator);

    assert_eq!(1, *first);
    assert_eq!(2, *second);
    assert_eq!(start, &*first as *const u64 as usize);
    assert_eq!(start + 8, &*second as *const u64 as usize);

    let mut vector = Vec::new_in(&allocator);
    vector.extend(0..4u32);
    assert_eq!(&[0, 1, 2, 3], vector.as_slice());

    // 256 bytes are not enough for this, so it fails instead of overrunning.
    assert!(Vec::<u8, _>::new_in(&allocator).try_reserve(1024).is_err());
}

#[cfg(test)]
fn memory_start<S: MemoryResourceStrategy>(allocator: &MemoryResourceAllocatorAdapter<S>) -> usize {
    allocator.inner.lock().memory.as_ptr() as usize
}
