use crate::memory::address::{MemoryAddress, MemoryAddressPerspective};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct MemoryRegion<P>
where
    P: MemoryAddressPerspective,
{
    start: MemoryAddress<P>,
    size: usize,
}
