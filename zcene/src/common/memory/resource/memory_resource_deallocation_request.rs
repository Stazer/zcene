use core::num::NonZero;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct MemoryResourceDeallocationRequest {
    position: usize,
    size: NonZero<usize>,
    alignment: NonZero<usize>,
}
