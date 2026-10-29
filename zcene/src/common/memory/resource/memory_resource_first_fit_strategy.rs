use crate::common::memory::resource::{
    MemoryResourceAllocationError, MemoryResourceAllocationRegion, MemoryResourceAllocationRequest,
    MemoryResourceDeallocationError, MemoryResourceDeallocationRequest, MemoryResourceStrategy,
};
use core::alloc::Layout;
use core::ptr::NonNull;
use linked_list_allocator::Heap;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct MemoryResourceFirstFitStrategy {
    heap: Heap,
}

impl Default for MemoryResourceFirstFitStrategy {
    fn default() -> Self {
        Self::new(Heap::empty())
    }
}

impl MemoryResourceStrategy for MemoryResourceFirstFitStrategy {
    fn initialize(&mut self, memory: &mut [u8]) {
        unsafe { self.heap.init(memory.as_mut_ptr(), memory.len()) }
    }

    fn allocate(
        &mut self,
        memory: &mut [u8],
        request: MemoryResourceAllocationRequest,
    ) -> Result<MemoryResourceAllocationRegion, MemoryResourceAllocationError> {
        let layout = Layout::from_size_align(
            usize::from(*request.size()),
            usize::from(*request.alignment()),
        )?;

        let data = match self.heap.allocate_first_fit(layout) {
            Ok(data) => data.as_ptr() as usize - memory.as_ptr() as usize,
            Err(()) => return Err(MemoryResourceAllocationError::Busy),
        };

        // Positions are offsets into `memory`, like with every other strategy.
        Ok(MemoryResourceAllocationRegion::new(
            data,
            0,
            0,
            *request.size(),
            0,
        ))
    }

    fn deallocate(
        &mut self,
        memory: &mut [u8],
        request: MemoryResourceDeallocationRequest,
    ) -> Result<(), MemoryResourceDeallocationError> {
        let layout = Layout::from_size_align(
            usize::from(*request.size()),
            usize::from(*request.alignment()),
        )?;

        if request.position() >= memory.len() {
            return Err(MemoryResourceDeallocationError::OutOfBounds);
        }

        unsafe {
            self.heap.deallocate(
                NonNull::new(memory.as_mut_ptr().add(request.position())).unwrap(),
                layout,
            );
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn allocate_deallocate() {
    use core::num::NonZero;

    #[repr(align(16))]
    struct Memory([u8; 256]);

    let mut memory = Memory([0u8; 256]);
    let mut strategy = MemoryResourceFirstFitStrategy::default();
    strategy.initialize(&mut memory.0);

    let request = || {
        MemoryResourceAllocationRequest::new(NonZero::new(32).unwrap(), NonZero::new(8).unwrap())
    };

    let first = strategy.allocate(&mut memory.0, request()).unwrap();
    let second = strategy.allocate(&mut memory.0, request()).unwrap();

    assert!(second.data_end().get() <= 256);
    assert_ne!(first.data_start(), second.data_start());

    strategy
        .deallocate(
            &mut memory.0,
            MemoryResourceDeallocationRequest::new(
                first.data_start(),
                NonZero::new(32).unwrap(),
                NonZero::new(8).unwrap(),
            ),
        )
        .unwrap();

    // The freed block is reused.
    assert_eq!(
        first.data_start(),
        strategy.allocate(&mut memory.0, request()).unwrap().data_start(),
    );
}
