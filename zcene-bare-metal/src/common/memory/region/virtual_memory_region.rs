use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddressTransformer, VirtualMemoryAddressPerspective,
};
use crate::common::memory::region::MemoryRegion;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type VirtualMemoryRegion<T = DefaultMemoryAddressTransformer>
    = MemoryRegion<VirtualMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
