use crate::memory::resource::{
    MemoryResourceAllocationError, MemoryResourceAllocationRegion, MemoryResourceAllocationRequest,
    MemoryResourceDeallocationError, MemoryResourceDeallocationRequest, MemoryResourceStrategy,
};
use core::alloc::Layout;
use core::num::NonZero;
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
        _memory: &mut [u8],
        request: MemoryResourceAllocationRequest,
    ) -> Result<MemoryResourceAllocationRegion, MemoryResourceAllocationError> {
        let layout = Layout::from_size_align(
            usize::from(*request.size()),
            usize::from(*request.alignment()),
        )?;

        let data = match self.heap.allocate_first_fit(layout) {
            Ok(data) => data.addr(),
            Err(()) => return Err(MemoryResourceAllocationError::Busy),
        };

        Ok(MemoryResourceAllocationRegion::new(
            usize::from(data),
            0,
            0,
            *request.size(),
            0,
        ))
    }

    fn deallocate(
        &mut self,
        _memory: &mut [u8],
        request: MemoryResourceDeallocationRequest,
    ) -> Result<(), MemoryResourceDeallocationError> {
        let layout = Layout::from_size_align(
            usize::from(*request.size()),
            usize::from(*request.alignment()),
        )?;

        unsafe {
            self.heap
                .deallocate(NonNull::new(request.position() as *mut u8).unwrap(), layout);
        }

        Ok(())
    }
}

/*////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct FirstFitAllocationStrategyAllocationHeader {
    allocated: bool,
    size: NonZero<usize>,
    previous_position: Option<usize>,
}

impl FirstFitAllocationStrategyAllocationHeader {
    pub fn new(allocated: bool, size: NonZero<usize>, previous_position: Option<usize>) -> Self {
        Self {
            allocated,
            size,
            previous_position,
        }
    }

    pub fn allocated(&self) -> bool {
        self.allocated
    }

    pub fn size(&self) -> NonZero<usize> {
        self.size
    }

    pub fn previous_position(&self) -> Option<usize> {
        self.previous_position
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct FirstFitAllocationStrategy {
    allocated_bytes: usize,
}

impl FirstFitAllocationStrategy {
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
    }

    fn header_at(
        memory: &mut [u8],
        position: usize,
    ) -> Option<&mut FirstFitAllocationStrategyAllocationHeader> {
        unsafe {
            Some(
                NonNull::new(memory.as_mut_ptr().wrapping_add(position)
                    as *mut FirstFitAllocationStrategyAllocationHeader)?
                .as_mut(),
            )
        }
    }

    fn update_next_previous_position(memory: &mut [u8], position: usize, size: NonZero<usize>) {
        if let Some(header) = Self::header_at(memory, position + usize::from(size)) {
            *header = FirstFitAllocationStrategyAllocationHeader::new(
                header.allocated(),
                header.size(),
                Some(position),
            );
        }
    }
}

impl AllocationStrategy for FirstFitAllocationStrategy {
    fn initialize(&mut self, memory: &mut [u8]) {
        let size = match NonZero::new(memory.len()) {
            Some(size) => size,
            None => unreachable!(),
        };

        let header = match Self::header_at(memory, 0) {
            Some(header) => header,
            None => unreachable!(),
        };

        *header = FirstFitAllocationStrategyAllocationHeader::new(false, size, None);
    }

    fn allocate(
        &mut self,
        memory: &mut [u8],
        request: AllocationRequest,
    ) -> Option<AllocationRegion> {
        let aligned_block_size = request.aligned_block_size(NonZero::new(size_of::<
            FirstFitAllocationStrategyAllocationHeader,
        >())?);

        let mut current_position = 0;

        while current_position < memory.len() {
            let current_header = match Self::header_at(memory, current_position) {
                Some(header) => header,
                None => unreachable!(),
            };

            if current_header.allocated() {
                current_position += usize::from(current_header.size());
                continue;
            }

            if current_header.size() < aligned_block_size {
                current_position += usize::from(current_header.size());
                continue;
            }

            self.allocated_bytes += usize::from(aligned_block_size);

            let remaining_size =
                usize::from(current_header.size()) - usize::from(aligned_block_size);
            let split_block =
                remaining_size > size_of::<FirstFitAllocationStrategyAllocationHeader>();

            let aligned_block_size = if split_block {
                aligned_block_size
            } else {
                current_header.size()
            };

            *current_header = FirstFitAllocationStrategyAllocationHeader::new(
                true,
                aligned_block_size,
                current_header.previous_position(),
            );

            if split_block {
                let next_header =
                    Self::header_at(memory, current_position + usize::from(aligned_block_size))
                        .unwrap();

                *next_header = FirstFitAllocationStrategyAllocationHeader::new(
                    next_header.allocated(),
                    NonZero::new(remaining_size).unwrap(),
                    Some(current_position),
                );
            }

            Self::update_next_previous_position(memory, current_position, aligned_block_size);

            return Some(AllocationRegion::new(
                current_position + size_of::<FirstFitAllocationStrategyAllocationHeader>(),
                request.size(),
            ));
        }

        None
    }

    fn deallocate(&mut self, memory: &mut [u8], region: AllocationRegion) {
        if region.start() > memory.len() {
            todo!()
        }

        if region.start() + usize::from(region.length()) > memory.len() {
            todo!()
        }

        let current_position =
            region.start() - size_of::<FirstFitAllocationStrategyAllocationHeader>();
        let current_header = match Self::header_at(memory, current_position) {
            Some(header) if header.allocated() => header.clone(),
            Some(_header) => todo!(),
            None => todo!(),
        };

        let next_free_position = current_position + usize::from(current_header.size());
        let next_free_header = match Self::header_at(memory, next_free_position) {
            Some(next_header) if !next_header.allocated() => Some(next_header.clone()),
            Some(_next_header) => None,
            None => None,
        };

        let previous_free_header = match current_header
            .previous_position()
            .and_then(|position| Self::header_at(memory, usize::from(position)))
        {
            Some(previous_header) if !previous_header.allocated() => Some(previous_header.clone()),
            Some(_next_header) => None,
            None => None,
        };

        let new_position = current_position
            - previous_free_header
                .as_ref()
                .map(|header| header.size())
                .map(usize::from)
                .unwrap_or_default();
        let new_size = previous_free_header
            .as_ref()
            .map(|header| header.size())
            .map(usize::from)
            .unwrap_or_default()
            + usize::from(current_header.size())
            + next_free_header
                .map(|header| header.size())
                .map(usize::from)
                .unwrap_or_default();

        let new_header = match Self::header_at(memory, usize::from(new_position)) {
            Some(header) => header,
            None => unreachable!(),
        };
        *new_header = FirstFitAllocationStrategyAllocationHeader::new(
            false,
            NonZero::new(new_size).unwrap(),
            previous_free_header
                .as_ref()
                .map(|header| header.previous_position())
                .unwrap_or_default(),
        );

        if let Some(next_header) = Self::header_at(memory, usize::from(new_position) + new_size) {
            *next_header = FirstFitAllocationStrategyAllocationHeader::new(
                next_header.allocated(),
                next_header.size(),
                Some(new_position),
            );
        };
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn allocate_deallocate() {
    let mut memory = [0u8; 128];
    let mut strategy = FirstFitAllocationStrategy::default();
    strategy.initialize(&mut memory);

    NonZero::new(8)
        .zip(NonZero::new(8))
        .map(|(size, alignment)| AllocationRequest::new(size, alignment))
        .and_then(|request| strategy.allocate(&mut memory, request));

    assert_eq!(40, strategy.allocated_bytes());
}
*/
