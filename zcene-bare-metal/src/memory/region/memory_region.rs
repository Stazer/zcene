use crate::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, MemoryAddressPerspective,
    MemoryAddressTransformer,
};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct MemoryRegion<P, T = DefaultMemoryAddressTransformer>
where
    P: MemoryAddressPerspective,
    T: MemoryAddressTransformer,
{
    start: MemoryAddress<P, T>,
    size: usize,
}
