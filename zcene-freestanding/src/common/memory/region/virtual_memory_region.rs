use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, VirtualMemoryAddressPerspective,
};
use crate::common::memory::region::MemoryRegion;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type VirtualMemoryRegion<T = DefaultMemoryAddressTransformer> =
    MemoryRegion<VirtualMemoryAddressPerspective, T>;
