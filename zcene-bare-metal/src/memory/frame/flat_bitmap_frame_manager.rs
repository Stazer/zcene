use crate::common::bits::{BitField, BitFieldLeftToRight};
use crate::memory::address::{MemoryAddress, MemoryAddressPerspective};
use crate::memory::frame::FrameIdentifier;
use core::marker::PhantomData;
use core::ops::Range;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum FrameManagerAllocationError {
    OutOfRange(FrameIdentifier),
    Allocated(FrameIdentifier),
    NotFeasible,
    Busy,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub enum FrameManagerDeallocationError {
    OutOfRange(FrameIdentifier),
    NotAllocated(FrameIdentifier),
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FlatBitmapFrameManager<'a, P>
where
    P: MemoryAddressPerspective,
{
    frame_byte_size: usize,
    slice: &'a mut [u8],
    memory_address_perspective: PhantomData<P>,
}

impl<'a, P> FlatBitmapFrameManager<'a, P>
where
    P: MemoryAddressPerspective,
{
    pub fn new(frame_byte_size: usize, slice: &'a mut [u8]) -> Self {
        let mut instance = Self {
            frame_byte_size,
            slice,
            memory_address_perspective: PhantomData::<P>,
        };

        instance.allocation_slice_mut().fill(0);

        instance
    }

    pub unsafe fn new_initialized(frame_byte_size: usize, slice: &'a mut [u8]) -> Self {
        Self {
            frame_byte_size,
            slice,
            memory_address_perspective: PhantomData::<P>,
        }
    }

    pub fn slice(&'a self) -> &'a [u8] {
        self.slice
    }

    pub fn slice_mut(&'a mut self) -> &'a mut [u8] {
        self.slice
    }

    pub fn frame_byte_size(&self) -> usize {
        self.frame_byte_size
    }

    #[inline]
    pub fn frame_bit_size(&self) -> usize {
        self.frame_byte_size * u8::BITS as usize
    }

    #[inline]
    pub fn translate_memory_address(&self, memory_address: MemoryAddress<P>) -> FrameIdentifier {
        FrameIdentifier::new(memory_address.as_usize() / self.frame_byte_size)
    }

    #[inline]
    pub fn translate_frame_identifier(
        &self,
        frame_identifier: FrameIdentifier,
    ) -> MemoryAddress<P> {
        MemoryAddress::new(frame_identifier.as_usize() * self.frame_byte_size)
    }

    #[inline]
    pub fn total_frame_count(&self) -> usize {
        self.slice.len() / self.frame_byte_size
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
    pub fn allocation_slice_length(&'a self) -> usize {
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

    pub fn allocation_slice(&self) -> &[u8] {
        let range = self.allocation_slice_range();

        &self.slice[range]
    }

    pub fn allocation_slice_mut(&mut self) -> &mut [u8] {
        let range = self.allocation_slice_range();

        &mut self.slice[range]
    }

    #[inline]
    pub fn data_slice_length(&'a self) -> usize {
        self.data_frame_count() * self.frame_byte_size()
    }

    #[inline]
    pub fn data_slice_start(&self) -> usize {
        0
    }

    #[inline]
    pub fn data_slice_end(&self) -> usize {
        self.data_slice_length()
    }

    pub fn data_slice_range(&self) -> Range<usize> {
        self.data_slice_start()..self.data_slice_end()
    }

    pub fn data_slice(&self) -> &[u8] {
        let range = self.data_slice_range();

        &self.slice[range]
    }

    pub fn data_slice_mut(&mut self) -> &mut [u8] {
        let range = self.data_slice_range();

        &mut self.slice[range]
    }

    pub fn allocation_frames(&self) -> impl Iterator<Item = &[u8]> {
        self.allocation_slice().chunks(self.frame_byte_size)
    }

    pub fn data_frames(&self) -> impl Iterator<Item = &[u8]> {
        self.data_slice().chunks(self.frame_byte_size)
    }

    pub fn frame_identifiers(&self) -> impl Iterator<Item = FrameIdentifier> + use<'_, P> {
        let length = {
            BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(self.allocation_slice())
                .len()
        };

        (0..length)
            .filter(|position| *position < self.data_frame_count())
            .map(FrameIdentifier::new)
    }

    pub fn allocated_frame_identifiers(
        &self,
    ) -> impl Iterator<Item = FrameIdentifier> + use<'_, P> {
        let field =
            BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(self.allocation_slice());

        self.frame_identifiers()
            .filter(move |identifier| field.is_enabled(identifier.as_usize()))
    }

    pub fn unallocated_frame_identifiers(
        &self,
    ) -> impl Iterator<Item = FrameIdentifier> + use<'_, P> {
        let field =
            BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(self.allocation_slice());

        self.frame_identifiers()
            .filter(move |identifier| field.is_disabled(identifier.as_usize()))
    }

    pub fn find_unallocated_continuum(
        &self,
        count: usize,
    ) -> Option<impl Iterator<Item = FrameIdentifier>> {
        let data_frame_count = self.data_frame_count();

        if data_frame_count < count {
            return None;
        }

        let field =
            BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(self.allocation_slice());

        let mut start = 0;

        while start < data_frame_count {
            let end = start + count;

            let mut allocatable = true;

            for current in start..end {
                if field.is_enabled(current) {
                    allocatable = false;
                    start = current + 1;

                    break;
                }
            }

            if allocatable {
                return Some((start..end).map(FrameIdentifier::new));
            }
        }

        None
    }

    pub fn allocate_window(
        &mut self,
        count: usize,
    ) -> Result<impl Iterator<Item = FrameIdentifier>, FrameManagerAllocationError> {
        let data_frame_count = self.data_frame_count();

        if data_frame_count < count {
            return Err(FrameManagerAllocationError::NotFeasible);
        }

        let mut field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice_mut(),
        );

        let mut start = 0;

        while start < data_frame_count {
            let end = start + count;

            let mut allocatable = true;

            for current in start..end {
                if field.is_enabled(current) {
                    allocatable = false;
                    start = current + 1;

                    break;
                }
            }

            if allocatable {
                for current in start..end {
                    field.enable(current)
                }

                return Ok((start..end).map(FrameIdentifier::new));
            }
        }

        Err(FrameManagerAllocationError::Busy)
    }

    pub fn allocate_frames<I>(&mut self, identifiers: I) -> Result<(), FrameManagerAllocationError>
    where
        I: Clone + Iterator<Item = FrameIdentifier>,
    {
        let data_frame_count = self.data_frame_count();

        let mut field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice_mut(),
        );

        for identifier in identifiers.clone() {
            if data_frame_count < identifier.as_usize() {
                return Err(FrameManagerAllocationError::OutOfRange(identifier));
            }

            if field.is_enabled(identifier.as_usize()) {
                return Err(FrameManagerAllocationError::Allocated(identifier));
            }
        }

        for identifier in identifiers {
            field.enable(identifier.as_usize());
        }

        Ok(())
    }

    pub fn deallocate_frames<I>(
        &mut self,
        identifiers: I,
    ) -> Result<(), FrameManagerDeallocationError>
    where
        I: Clone + Iterator<Item = FrameIdentifier>,
    {
        let data_frame_count = self.data_frame_count();

        let mut field = BitField::<BitFieldLeftToRight, BitFieldLeftToRight, _>::new(
            self.allocation_slice_mut(),
        );

        for identifier in identifiers.clone() {
            if data_frame_count < identifier.as_usize() {
                return Err(FrameManagerDeallocationError::OutOfRange(identifier));
            }

            if field.is_disabled(identifier.as_usize()) {
                return Err(FrameManagerDeallocationError::NotAllocated(identifier));
            }
        }

        for identifier in identifiers {
            field.disable(identifier.as_usize());
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
use crate::common::memory::UnknownMemoryAddressPerspective;
#[cfg(test)]
use std::vec::Vec;

#[test]
#[cfg(test)]
fn translate_memory_address() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        4,
        manager
            .translate_memory_address(MemoryAddress::new(18))
            .as_usize(),
    );
}

#[test]
fn translate_frame_identifier() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        16,
        manager
            .translate_frame_identifier(FrameIdentifier::new(4))
            .as_usize(),
    );
}

#[test]
#[cfg(test)]
fn total_frame_count() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(256, manager.total_frame_count());
}

#[test]
#[cfg(test)]
fn allocation_frame_count() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(8, manager.allocation_frame_count());
}

#[test]
fn data_frame_count() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(248, manager.data_frame_count());
}

#[test]
fn allocation_frame_initialized_with_zero() {
    let mut memory: [u8; 1024] = [1; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert!(manager.allocation_slice().iter().all(|x| *x == 0));
}

#[test]
fn allocation_slice_range() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(992..1024, manager.allocation_slice_range());
}

#[test]
#[cfg(test)]
fn data_slice_range() {
    let mut memory: [u8; 1024] = [0; 1024];
    let manager = FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(0..992, manager.data_slice_range());
}

#[test]
#[cfg(test)]
fn allocate_window() {
    let mut memory: [u8; 1024] = [0; 1024];
    let mut manager =
        FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        Ok((0..4).collect::<Vec<usize>>()),
        manager.allocate_window(4).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );

    assert_eq!(
        Ok((4..6).collect::<Vec<usize>>()),
        manager.allocate_window(2).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );

    assert_eq!(
        (0..6).collect::<Vec<usize>>(),
        manager
            .allocated_frame_identifiers()
            .map(|identifier| identifier.as_usize())
            .collect::<Vec<usize>>(),
    );

    assert_eq!(
        (0..(manager.data_frame_count()))
            .skip(6)
            .collect::<Vec<usize>>(),
        manager
            .unallocated_frame_identifiers()
            .map(|identifier| identifier.as_usize())
            .collect::<Vec<usize>>(),
    );
}

#[test]
#[cfg(test)]
fn allocate_window_bounds_checking() {
    let mut memory: [u8; 1024] = [0; 1024];
    let mut manager =
        FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        Err(FrameManagerAllocationError::NotFeasible),
        manager
            .allocate_window(manager.data_frame_count() + 1)
            .map(|identifiers| {
                identifiers
                    .map(|identifier| identifier.as_usize())
                    .collect::<Vec<usize>>()
            })
    );

    assert!(manager
        .allocate_window(manager.data_frame_count())
        .map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
        .is_ok());

    assert_eq!(
        Err(FrameManagerAllocationError::Busy),
        manager.allocate_window(1).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );
}

#[test]
fn allocate_window_empty() {
    let mut memory: [u8; 1024] = [0; 1024];
    let mut manager =
        FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        Ok((0..0).collect::<Vec<usize>>()),
        manager.allocate_window(0).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );
}

#[test]
#[cfg(test)]
fn deallocate_frames() {
    let mut memory: [u8; 1024] = [0; 1024];
    let mut manager =
        FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        Ok((0..4).collect::<Vec<usize>>()),
        manager.allocate_window(4).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );

    assert_eq!(
        Ok(()),
        manager.deallocate_frames([0, 3].iter().copied().map(FrameIdentifier::new))
    );

    assert_eq!(
        Ok((3..5).collect::<Vec<usize>>()),
        manager.allocate_window(2).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );
}

#[test]
#[cfg(test)]
fn reallocation() {
    let mut memory: [u8; 1024] = [0; 1024];
    let mut manager =
        FlatBitmapFrameManager::<UnknownMemoryAddressPerspective>::new(4, &mut memory);

    assert_eq!(
        Ok((0..4).collect::<Vec<usize>>()),
        manager.allocate_window(4).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );

    assert_eq!(
        Ok(()),
        manager.deallocate_frames([0, 2].iter().copied().map(FrameIdentifier::new))
    );

    assert_eq!(
        Ok((4..6).collect::<Vec<usize>>()),
        manager.allocate_window(2).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );

    assert_eq!(
        Ok((0..1).collect::<Vec<usize>>()),
        manager.allocate_window(1).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );

    assert_eq!(
        Ok((2..3).collect::<Vec<usize>>()),
        manager.allocate_window(1).map(|identifiers| {
            identifiers
                .map(|identifier| identifier.as_usize())
                .collect::<Vec<usize>>()
        })
    );
}
