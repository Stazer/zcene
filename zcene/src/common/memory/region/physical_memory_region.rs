use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, PhysicalMemoryAddressPerspective,
};
use crate::common::memory::region::MemoryRegion;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type PhysicalMemoryRegion<T = DefaultMemoryAddressTransformer> =
    MemoryRegion<PhysicalMemoryAddressPerspective, T>;
