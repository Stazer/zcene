use crate::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddressTransformer, PhysicalMemoryAddressPerspective,
};
use crate::memory::region::MemoryRegion;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type PhysicalMemoryRegion<T = DefaultMemoryAddressTransformer>
    = MemoryRegion<PhysicalMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
