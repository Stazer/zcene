use crate::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddressTransformer, VirtualMemoryAddressPerspective,
};
use crate::memory::region::MemoryRegion;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type VirtualMemoryRegion<T = DefaultMemoryAddressTransformer>
    = MemoryRegion<VirtualMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
