use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddressTransformer, PhysicalMemoryAddressPerspective,
};
use crate::common::memory::region::MemoryRegion;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type PhysicalMemoryRegion<T = DefaultMemoryAddressTransformer>
    = MemoryRegion<PhysicalMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
