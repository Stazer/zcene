use crate::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, MemoryAddressTransformer,
    PhysicalMemoryAddressPerspective,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type PhysicalMemoryAddress<T = DefaultMemoryAddressTransformer>
    = MemoryAddress<PhysicalMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
