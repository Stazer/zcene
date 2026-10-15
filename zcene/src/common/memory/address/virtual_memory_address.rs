use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, VirtualMemoryAddressPerspective,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type VirtualMemoryAddress<T = DefaultMemoryAddressTransformer> =
    MemoryAddress<VirtualMemoryAddressPerspective, T>;
