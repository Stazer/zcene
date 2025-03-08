use crate::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, MemoryAddressTransformer,
    UnknownMemoryAddressPerspective,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type UnknownMemoryAddress<T = DefaultMemoryAddressTransformer>
    = MemoryAddress<UnknownMemoryAddressPerspective, T>
where
    T: MemoryAddressTransformer;
