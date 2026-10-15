use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, PhysicalMemoryAddressPerspective,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type PhysicalMemoryAddress<T = DefaultMemoryAddressTransformer> =
    MemoryAddress<PhysicalMemoryAddressPerspective, T>;
