use core::num::NonZero;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct MemoryResourceAllocationRequest {
    size: NonZero<usize>,
    alignment: NonZero<usize>,
}
