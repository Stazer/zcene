use crate::common::memory::allocator::FrameManagerAllocationError;
use core::alloc::AllocError;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::page::AddressNotAligned;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum X86_64MemoryManagerInitializeError {
    UnsupportedMapping,
    FrameAllocation(FrameManagerAllocationError),
    AddressNotAligned(AddressNotAligned),
    MapToErrorFrameAllocationFailed,
    MapToErrorParentEntryHugePage,
    MapToErrorPageAlreadyMapped,
    AllocError(AllocError),
}

impl From<AddressNotAligned> for X86_64MemoryManagerInitializeError {
    fn from(error: AddressNotAligned) -> Self {
        Self::AddressNotAligned(error)
    }
}

impl From<FrameManagerAllocationError> for X86_64MemoryManagerInitializeError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl From<AllocError> for X86_64MemoryManagerInitializeError {
    fn from(error: AllocError) -> Self {
        Self::AllocError(error)
    }
}

impl<P> From<MapToError<P>> for X86_64MemoryManagerInitializeError
where
    P: PageSize,
{
    fn from(error: MapToError<P>) -> Self {
        match error {
            MapToError::FrameAllocationFailed => Self::MapToErrorFrameAllocationFailed,
            MapToError::ParentEntryHugePage => Self::MapToErrorParentEntryHugePage,
            MapToError::PageAlreadyMapped(_) => Self::MapToErrorPageAlreadyMapped,
        }
    }
}
