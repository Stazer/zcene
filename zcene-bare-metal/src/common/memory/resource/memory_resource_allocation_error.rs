use core::alloc::LayoutError;
use ztd::{Display, Error};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Display, Error, PartialEq)]
pub enum MemoryResourceAllocationError {
    Busy,
    InvalidLayout,
}

impl From<LayoutError> for MemoryResourceAllocationError {
    fn from(_error: LayoutError) -> Self {
        Self::InvalidLayout
    }
}
