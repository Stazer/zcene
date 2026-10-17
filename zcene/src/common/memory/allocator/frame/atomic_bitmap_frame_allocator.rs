use crate::common::bits::{AtomicBitField, BitFieldLeftToRight};
use crate::common::memory::address::{MemoryAddress, MemoryAddressPerspective};
use crate::common::memory::FrameIdentifier;
use crate::common::memory::FrameIdentifierMemoryAddressTranslator;
use core::fmt::{self, Debug, Formatter};
use core::marker::PhantomData;
use core::ops::Range;
use core::sync::atomic::AtomicU8;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum AtomicBitmapFrameAllocatorAllocationError {
    OutOfRange(FrameIdentifier),
    Allocated(FrameIdentifier),
    NotFeasible,
    Busy,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum AtomicBitmapFrameAllocatorDeallocationError {
    OutOfRange(FrameIdentifier),
    NotAllocated(FrameIdentifier),
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Lock-free counterpart of [`crate::common::memory::allocator::FlatBitmapFrameAllocator`]: the
/// same flat-bitmap-at-the-tail-of-the-slice layout, but every allocate/deallocate is a single
/// atomic RMW on the bit's own byte (see [`AtomicBitField`]), so it is race-free under concurrent
/// access from multiple cores through a shared `&self` — no lock, no exclusive access needed.
pub struct AtomicBitmapFrameAllocator<'a, P>
where
    P: MemoryAddressPerspective,
{
    frame_identifier_memory_address_translator: FrameIdentifierMemoryAddressTranslator,
    slice: &'a [AtomicU8],
    memory_address_perspective: PhantomData<P>,
}

impl<P> Debug for AtomicBitmapFrameAllocator<'_, P>
where
    P: MemoryAddressPerspective,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AtomicBitmapFrameAllocator")
            .field(
                "frame_identifier_memory_address_translator",
                &self.frame_identifier_memory_address_translator,
            )
            .finish_non_exhaustive()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'a, P> AtomicBitmapFrameAllocator<'a, P>
where
    P: MemoryAddressPerspective,
{
    /// `allocated_frame_identifiers` is applied with the plain, non-exclusive `enable` — this is
    /// meant to run once, before any other core could possibly race against it (the same
    /// precondition `FlatBitmapFrameAllocator::new` has), so there is nothing to detect a
    /// conflict against yet.
    pub fn new<I>(
        frame_byte_size: usize,
        slice: &'a [AtomicU8],
        allocated_frame_identifiers: I,
    ) -> Result<Self, AtomicBitmapFrameAllocatorAllocationError>
    where
        I: IntoIterator<Item = FrameIdentifier>,
    {
        let instance = Self {
            frame_identifier_memory_address_translator: FrameIdentifierMemoryAddressTranslator::new(
                frame_byte_size,
            ),
            slice,
            memory_address_perspective: PhantomData::<P>,
        };

        for byte in instance.allocation_slice() {
            byte.store(0, core::sync::atomic::Ordering::Relaxed);
        }

        let data_frame_count = instance.data_frame_count();
        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            instance.allocation_slice(),
        );

        for identifier in allocated_frame_identifiers {
            if data_frame_count < identifier.as_usize() {
                return Err(AtomicBitmapFrameAllocatorAllocationError::OutOfRange(identifier));
            }

            field.enable(identifier.as_usize());
        }

        Ok(instance)
    }

    pub fn frame_byte_size(&self) -> usize {
        self.frame_identifier_memory_address_translator.frame_size()
    }

    #[inline]
    pub fn frame_bit_size(&self) -> usize {
        self.frame_byte_size() * u8::BITS as usize
    }

    #[inline]
    pub fn translate_memory_address(&self, memory_address: MemoryAddress<P>) -> FrameIdentifier {
        self.frame_identifier_memory_address_translator
            .translate_memory_address(memory_address)
    }

    #[inline]
    pub fn translate_frame_identifier(
        &self,
        frame_identifier: FrameIdentifier,
    ) -> MemoryAddress<P> {
        self.frame_identifier_memory_address_translator
            .translate_frame_identifier(frame_identifier)
    }

    #[inline]
    pub fn total_frame_count(&self) -> usize {
        self.slice.len() / self.frame_byte_size()
    }

    #[inline]
    pub fn allocation_frame_count(&self) -> usize {
        self.total_frame_count() / self.frame_bit_size()
    }

    #[inline]
    pub fn data_frame_count(&self) -> usize {
        self.total_frame_count() - self.allocation_frame_count()
    }

    #[inline]
    pub fn allocation_slice_length(&self) -> usize {
        self.allocation_frame_count() * self.frame_byte_size()
    }

    #[inline]
    pub fn allocation_slice_start(&self) -> usize {
        self.slice.len() - self.allocation_slice_length()
    }

    #[inline]
    pub fn allocation_slice_end(&self) -> usize {
        self.slice.len()
    }

    pub fn allocation_slice_range(&self) -> Range<usize> {
        self.allocation_slice_start()..self.allocation_slice_end()
    }

    pub fn allocation_slice(&self) -> &'a [AtomicU8] {
        &self.slice[self.allocation_slice_range()]
    }

    #[inline]
    pub fn data_slice_length(&self) -> usize {
        self.data_frame_count() * self.frame_byte_size()
    }

    pub fn data_slice_range(&self) -> Range<usize> {
        0..self.data_slice_length()
    }

    pub fn data_slice(&self) -> &'a [AtomicU8] {
        &self.slice[self.data_slice_range()]
    }

    pub fn frame_identifiers(&self) -> impl Iterator<Item = FrameIdentifier> + use<'_, 'a, P> {
        let length = {
            AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
                self.allocation_slice(),
            )
            .len()
        };

        (0..length)
            .filter(|position| *position < self.data_frame_count())
            .map(FrameIdentifier::new)
    }

    pub fn allocated_frame_identifiers(
        &self,
    ) -> impl Iterator<Item = FrameIdentifier> + use<'_, 'a, P> {
        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice(),
        );

        self.frame_identifiers()
            .filter(move |identifier| field.is_enabled(identifier.as_usize()))
    }

    pub fn unallocated_frame_identifiers(
        &self,
    ) -> impl Iterator<Item = FrameIdentifier> + use<'_, 'a, P> {
        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice(),
        );

        self.frame_identifiers()
            .filter(move |identifier| field.is_disabled(identifier.as_usize()))
    }

    /// Claims a single, previously unallocated frame. Scans from the start of the data range
    /// every time — a per-core free cursor/cache is a concern for a layer above this one, not
    /// this bottom-most primitive.
    pub fn allocate_frame(&self) -> Result<FrameIdentifier, AtomicBitmapFrameAllocatorAllocationError> {
        let data_frame_count = self.data_frame_count();
        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice(),
        );

        for position in 0..data_frame_count {
            if field.exclusive_enable(position) {
                return Ok(FrameIdentifier::new(position));
            }
        }

        Err(AtomicBitmapFrameAllocatorAllocationError::Busy)
    }

    /// Claims a specific, caller-chosen set of frames (e.g. a fixed low frame reserved by the
    /// linker). Unlike `FlatBitmapFrameAllocator::allocate_frames`, this cannot validate the
    /// whole set before applying any of it — under concurrent access there is no such thing as
    /// "the state before I touched it" shared across multiple bits. Instead, each identifier is
    /// claimed one at a time, and on the first conflict every identifier already claimed by this
    /// call (never ones claimed by anyone else) is rolled back, leaving no net change.
    pub fn allocate_frames<I>(
        &self,
        identifiers: I,
    ) -> Result<(), AtomicBitmapFrameAllocatorAllocationError>
    where
        I: Iterator<Item = FrameIdentifier> + Clone,
    {
        let data_frame_count = self.data_frame_count();
        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice(),
        );

        let mut claimed_count = 0usize;

        for identifier in identifiers.clone() {
            if data_frame_count < identifier.as_usize() {
                for rollback_identifier in identifiers.clone().take(claimed_count) {
                    field.disable(rollback_identifier.as_usize());
                }

                return Err(AtomicBitmapFrameAllocatorAllocationError::OutOfRange(identifier));
            }

            if !field.exclusive_enable(identifier.as_usize()) {
                for rollback_identifier in identifiers.clone().take(claimed_count) {
                    field.disable(rollback_identifier.as_usize());
                }

                return Err(AtomicBitmapFrameAllocatorAllocationError::Allocated(identifier));
            }

            claimed_count += 1;
        }

        Ok(())
    }

    /// Claims `count` contiguous, previously unallocated frames. Candidates are tried left to
    /// right; on a conflict, everything claimed for that candidate window is rolled back (we
    /// know we exclusively own exactly those bits, so a plain `disable` is enough — no need to
    /// re-check), and the search continues right after the conflicting frame.
    pub fn allocate_window(
        &self,
        count: usize,
    ) -> Result<impl Iterator<Item = FrameIdentifier>, AtomicBitmapFrameAllocatorAllocationError> {
        let data_frame_count = self.data_frame_count();

        if data_frame_count < count {
            return Err(AtomicBitmapFrameAllocatorAllocationError::NotFeasible);
        }

        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice(),
        );

        let mut start = 0;

        while start + count <= data_frame_count {
            let end = start + count;
            let mut claimed_up_to = start;
            let mut conflict = None;

            for current in start..end {
                if field.exclusive_enable(current) {
                    claimed_up_to = current + 1;
                } else {
                    conflict = Some(current);
                    break;
                }
            }

            let Some(conflict_position) = conflict else {
                return Ok((start..end).map(FrameIdentifier::new));
            };

            for rollback in start..claimed_up_to {
                field.disable(rollback);
            }

            start = conflict_position + 1;
        }

        Err(AtomicBitmapFrameAllocatorAllocationError::Busy)
    }

    /// Frees a single frame, detecting a double free (the bit was already clear) rather than
    /// silently accepting it.
    pub fn deallocate_frame(
        &self,
        identifier: FrameIdentifier,
    ) -> Result<(), AtomicBitmapFrameAllocatorDeallocationError> {
        let data_frame_count = self.data_frame_count();

        if data_frame_count < identifier.as_usize() {
            return Err(AtomicBitmapFrameAllocatorDeallocationError::OutOfRange(identifier));
        }

        let field = AtomicBitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice(),
        );

        if field.exclusive_disable(identifier.as_usize()) {
            Ok(())
        } else {
            Err(AtomicBitmapFrameAllocatorDeallocationError::NotAllocated(identifier))
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
use crate::common::memory::address::UnknownMemoryAddressPerspective;
#[cfg(test)]
use alloc::vec::Vec;

#[cfg(test)]
fn test_allocator(
    slice: &[AtomicU8],
) -> AtomicBitmapFrameAllocator<'_, UnknownMemoryAddressPerspective> {
    AtomicBitmapFrameAllocator::new(4, slice, []).unwrap()
}

#[test]
#[cfg(test)]
fn total_frame_count() {
    let memory: [AtomicU8; 1024] = core::array::from_fn(|_| AtomicU8::new(0));
    let allocator = test_allocator(&memory);

    assert_eq!(256, allocator.total_frame_count());
}

#[test]
#[cfg(test)]
fn data_frame_count() {
    let memory: [AtomicU8; 1024] = core::array::from_fn(|_| AtomicU8::new(0));
    let allocator = test_allocator(&memory);

    assert_eq!(248, allocator.data_frame_count());
}

#[test]
#[cfg(test)]
fn allocate_frame_claims_distinct_frames() {
    let memory: [AtomicU8; 1024] = core::array::from_fn(|_| AtomicU8::new(0));
    let allocator = test_allocator(&memory);

    let first = allocator.allocate_frame().unwrap();
    let second = allocator.allocate_frame().unwrap();

    assert_ne!(first, second);
    assert!(allocator.allocated_frame_identifiers().any(|id| id == first));
    assert!(allocator.allocated_frame_identifiers().any(|id| id == second));
}

#[test]
#[cfg(test)]
fn allocate_frames_rolls_back_on_conflict() {
    let memory: [AtomicU8; 1024] = core::array::from_fn(|_| AtomicU8::new(0));
    let allocator = test_allocator(&memory);

    let pre_allocated = FrameIdentifier::new(2);
    allocator
        .allocate_frames([pre_allocated].into_iter())
        .unwrap();

    let attempt = [FrameIdentifier::new(0), FrameIdentifier::new(1), pre_allocated];

    assert_eq!(
        Err(AtomicBitmapFrameAllocatorAllocationError::Allocated(pre_allocated)),
        allocator.allocate_frames(attempt.into_iter()),
    );

    // The two frames claimed before the conflict must have been rolled back.
    assert!(allocator.unallocated_frame_identifiers().any(|id| id == FrameIdentifier::new(0)));
    assert!(allocator.unallocated_frame_identifiers().any(|id| id == FrameIdentifier::new(1)));
}

#[test]
#[cfg(test)]
fn allocate_window_skips_conflicting_region() {
    let memory: [AtomicU8; 1024] = core::array::from_fn(|_| AtomicU8::new(0));
    let allocator = test_allocator(&memory);

    allocator
        .allocate_frames([FrameIdentifier::new(2)].into_iter())
        .unwrap();

    let window = allocator
        .allocate_window(4)
        .unwrap()
        .map(|identifier| identifier.as_usize())
        .collect::<Vec<usize>>();

    assert_eq!((3..7).collect::<Vec<usize>>(), window);

    // The frames tentatively claimed at [0, 1] while probing must have been rolled back.
    assert!(allocator.unallocated_frame_identifiers().any(|id| id == FrameIdentifier::new(0)));
    assert!(allocator.unallocated_frame_identifiers().any(|id| id == FrameIdentifier::new(1)));
}

#[test]
#[cfg(test)]
fn deallocate_frame_detects_double_free() {
    let memory: [AtomicU8; 1024] = core::array::from_fn(|_| AtomicU8::new(0));
    let allocator = test_allocator(&memory);

    let frame = allocator.allocate_frame().unwrap();

    assert_eq!(Ok(()), allocator.deallocate_frame(frame));
    assert_eq!(
        Err(AtomicBitmapFrameAllocatorDeallocationError::NotAllocated(frame)),
        allocator.deallocate_frame(frame),
    );
}
