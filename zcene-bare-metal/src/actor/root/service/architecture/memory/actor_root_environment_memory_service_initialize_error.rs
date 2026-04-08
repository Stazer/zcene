use core::alloc::AllocError;
use crate::common::memory::allocator::FrameManagerAllocationError;
use x86_64::structures::paging::PageSize;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::page::AddressNotAligned;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum ActorRootEnvironmentMemoryServiceInitializeError {
    UnsupportedMapping,
    FrameAllocation(FrameManagerAllocationError),
    AddressNotAligned(AddressNotAligned),
    MapToErrorFrameAllocationFailed,
    MapToErrorParentEntryHugePage,
    MapToErrorPageAlreadyMapped,
    AllocError(AllocError),
}

impl From<AddressNotAligned> for ActorRootEnvironmentMemoryServiceInitializeError{
    fn from(error: AddressNotAligned) -> Self {
        Self::AddressNotAligned(error)
    }
}

impl From<FrameManagerAllocationError> for ActorRootEnvironmentMemoryServiceInitializeError{
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl From<AllocError> for ActorRootEnvironmentMemoryServiceInitializeError{
    fn from(error: AllocError) -> Self {
        Self::AllocError(error)
    }
}

impl<P> From<MapToError<P>> for ActorRootEnvironmentMemoryServiceInitializeError
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
