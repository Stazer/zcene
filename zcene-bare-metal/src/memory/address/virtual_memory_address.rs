use crate::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, MemoryAddressTransformer,
    VirtualMemoryAddressPerspective,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type VirtualMemoryAddress<T = DefaultMemoryAddressTransformer>
    = MemoryAddress<VirtualMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
