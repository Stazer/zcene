use core::alloc::LayoutError;
use ztd::{Display, Error};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Display, Error, PartialEq)]
pub enum MemoryResourceDeallocationError {
    OutOfBounds,
    NotAllocated,
    InvalidLayout,
}

impl From<LayoutError> for MemoryResourceDeallocationError {
    fn from(_error: LayoutError) -> Self {
        Self::InvalidLayout
    }
}
