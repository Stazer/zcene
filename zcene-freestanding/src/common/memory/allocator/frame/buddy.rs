use crate::common::memory::address::{MemoryAddress, MemoryAddressPerspective};
use crate::common::memory::allocator::frame::FrameIdentifier;
use core::marker::PhantomData;
use core::mem::size_of;

////////////////////////////////////////////////////////////////////////////////////////////////////

// Sentinel: no next free block.
const NONE: usize = usize::MAX;

// Maximum supported order (handles up to 2^48 frames).
const MAX_ORDER: usize = 48;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum BuddyFrameAllocatorAllocationError {
    OutOfMemory,
    InvalidOrder,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum BuddyFrameAllocatorDeallocationError {
    InvalidOrder,
    OutOfRange(FrameIdentifier),
}

////////////////////////////////////////////////////////////////////////////////////////////////////

// Buddy frame allocator. Free list nodes are embedded in the first `size_of::<usize>()` bytes
// of each free block, so no external allocation is needed. `frame_byte_size` must be at least
// `size_of::<usize>()`. Total allocatable frames is rounded down to the nearest power of two.
pub struct BuddyFrameAllocator<'a, P>
where
    P: MemoryAddressPerspective,
{
    frame_byte_size: usize,
    max_order: usize,
    free_list_heads: [usize; MAX_ORDER + 1],
    slice: &'a mut [u8],
    _phantom: PhantomData<P>,
}

impl<'a, P> BuddyFrameAllocator<'a, P>
where
    P: MemoryAddressPerspective,
{
    pub fn new(frame_byte_size: usize, slice: &'a mut [u8]) -> Self {
        assert!(frame_byte_size >= size_of::<usize>());
        let total_frames = slice.len() / frame_byte_size;
        assert!(total_frames > 0);

        let max_order = total_frames.ilog2() as usize;
        let mut instance = Self {
            frame_byte_size,
            max_order,
            free_list_heads: [NONE; MAX_ORDER + 1],
            slice,
            _phantom: PhantomData,
        };

        instance.push_free(max_order, 0);
        instance
    }

    pub fn frame_byte_size(&self) -> usize {
        self.frame_byte_size
    }

    pub fn max_order(&self) -> usize {
        self.max_order
    }

    pub fn total_frames(&self) -> usize {
        1 << self.max_order
    }

    pub fn translate_frame_identifier(&self, frame: FrameIdentifier) -> MemoryAddress<P> {
        MemoryAddress::new(frame.as_usize() * self.frame_byte_size)
    }

    pub fn translate_memory_address(&self, address: MemoryAddress<P>) -> FrameIdentifier {
        FrameIdentifier::new(address.as_usize() / self.frame_byte_size)
    }

    // Allocates a block of 2^order contiguous frames, returning the base FrameIdentifier.
    pub fn allocate_frame(
        &mut self,
        order: usize,
    ) -> Result<FrameIdentifier, BuddyFrameAllocatorAllocationError> {
        if order > self.max_order {
            return Err(BuddyFrameAllocatorAllocationError::InvalidOrder);
        }

        let found_order = (order..=self.max_order)
            .find(|&k| self.free_list_heads[k] != NONE)
            .ok_or(BuddyFrameAllocatorAllocationError::OutOfMemory)?;

        let block = self.pop_free(found_order).unwrap();

        // Split the block down, freeing the upper half at each level.
        for k in (order..found_order).rev() {
            self.push_free(k, block + (1 << k));
        }

        Ok(FrameIdentifier::new(block))
    }

    // Deallocates the block of 2^order frames at `frame`, merging with its buddy when free.
    pub fn deallocate_frame(
        &mut self,
        frame: FrameIdentifier,
        order: usize,
    ) -> Result<(), BuddyFrameAllocatorDeallocationError> {
        if order > self.max_order {
            return Err(BuddyFrameAllocatorDeallocationError::InvalidOrder);
        }

        let mut current = frame.as_usize();

        if current >= self.total_frames() {
            return Err(BuddyFrameAllocatorDeallocationError::OutOfRange(frame));
        }

        let mut current_order = order;

        while current_order < self.max_order {
            let buddy = current ^ (1 << current_order);
            if self.remove_free(current_order, buddy) {
                current = current.min(buddy);
                current_order += 1;
            } else {
                break;
            }
        }

        self.push_free(current_order, current);
        Ok(())
    }

    fn read_next(&self, frame_index: usize) -> usize {
        let offset = frame_index * self.frame_byte_size;
        let mut bytes = [0u8; size_of::<usize>()];
        bytes.copy_from_slice(&self.slice[offset..offset + size_of::<usize>()]);
        usize::from_ne_bytes(bytes)
    }

    fn write_next(&mut self, frame_index: usize, next: usize) {
        let offset = frame_index * self.frame_byte_size;
        self.slice[offset..offset + size_of::<usize>()].copy_from_slice(&next.to_ne_bytes());
    }

    fn push_free(&mut self, order: usize, frame_index: usize) {
        let head = self.free_list_heads[order];
        self.write_next(frame_index, head);
        self.free_list_heads[order] = frame_index;
    }

    fn pop_free(&mut self, order: usize) -> Option<usize> {
        let head = self.free_list_heads[order];
        if head == NONE {
            return None;
        }
        self.free_list_heads[order] = self.read_next(head);
        Some(head)
    }

    // Removes `frame_index` from the free list at `order`. Returns true if found.
    fn remove_free(&mut self, order: usize, frame_index: usize) -> bool {
        let mut prev = NONE;
        let mut current = self.free_list_heads[order];

        while current != NONE {
            let next = self.read_next(current);
            if current == frame_index {
                if prev == NONE {
                    self.free_list_heads[order] = next;
                } else {
                    self.write_next(prev, next);
                }
                return true;
            }
            prev = current;
            current = next;
        }

        false
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
use crate::common::memory::address::UnknownMemoryAddressPerspective;

// Use the smallest valid frame size so test stack buffers stay tiny.
#[cfg(test)]
const TEST_FRAME_SIZE: usize = size_of::<usize>();

#[test]
#[cfg(test)]
fn allocate_single_frame() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let frame = allocator.allocate_frame(0).unwrap();
    assert_eq!(0, frame.as_usize());
}

#[test]
#[cfg(test)]
fn allocate_second_frame_after_split() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    allocator.allocate_frame(0).unwrap();
    let frame = allocator.allocate_frame(0).unwrap();
    assert_eq!(1, frame.as_usize());
}

#[test]
#[cfg(test)]
fn allocate_order_block_returns_aligned_base() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let frame = allocator.allocate_frame(2).unwrap();
    assert_eq!(0, frame.as_usize());
    let frame2 = allocator.allocate_frame(2).unwrap();
    assert_eq!(4, frame2.as_usize());
}

#[test]
#[cfg(test)]
fn allocate_fills_all_frames_then_oom() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    for _ in 0..16 {
        assert!(allocator.allocate_frame(0).is_ok());
    }
    assert_eq!(
        Err(BuddyFrameAllocatorAllocationError::OutOfMemory),
        allocator.allocate_frame(0),
    );
}

#[test]
#[cfg(test)]
fn allocate_invalid_order_rejected() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let over_max = allocator.max_order() + 1;
    assert_eq!(
        Err(BuddyFrameAllocatorAllocationError::InvalidOrder),
        allocator.allocate_frame(over_max),
    );
}

#[test]
#[cfg(test)]
fn deallocate_then_reallocate() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let frame = allocator.allocate_frame(0).unwrap();
    allocator.deallocate_frame(frame, 0).unwrap();
    let reallocated = allocator.allocate_frame(0).unwrap();
    assert_eq!(frame, reallocated);
}

#[test]
#[cfg(test)]
fn deallocate_merges_buddies_back_to_max_order() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let max_order = allocator.max_order();

    let mut frames = [FrameIdentifier::new(0); 16];
    for f in frames.iter_mut() {
        *f = allocator.allocate_frame(0).unwrap();
    }
    for f in frames {
        allocator.deallocate_frame(f, 0).unwrap();
    }
    // All buddies merged — full block should be available again.
    assert!(allocator.allocate_frame(max_order).is_ok());
}

#[test]
#[cfg(test)]
fn deallocate_out_of_range_rejected() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let bad = FrameIdentifier::new(allocator.total_frames());
    assert_eq!(
        Err(BuddyFrameAllocatorDeallocationError::OutOfRange(bad)),
        allocator.deallocate_frame(bad, 0),
    );
}

#[test]
#[cfg(test)]
fn deallocate_invalid_order_rejected() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let mut allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    let frame = allocator.allocate_frame(0).unwrap();
    let over_max = allocator.max_order() + 1;
    assert_eq!(
        Err(BuddyFrameAllocatorDeallocationError::InvalidOrder),
        allocator.deallocate_frame(frame, over_max),
    );
}

#[test]
#[cfg(test)]
fn translate_frame_identifier() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    assert_eq!(
        TEST_FRAME_SIZE * 2,
        allocator
            .translate_frame_identifier(FrameIdentifier::new(2))
            .as_usize(),
    );
}

#[test]
#[cfg(test)]
fn translate_memory_address() {
    let mut memory = [0u8; TEST_FRAME_SIZE * 16];
    let allocator =
        BuddyFrameAllocator::<UnknownMemoryAddressPerspective>::new(TEST_FRAME_SIZE, &mut memory);
    assert_eq!(
        2,
        allocator
            .translate_memory_address(MemoryAddress::new(TEST_FRAME_SIZE * 2))
            .as_usize(),
    );
}
