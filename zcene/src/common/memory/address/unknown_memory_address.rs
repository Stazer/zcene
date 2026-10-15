use crate::common::memory::address::{
    DefaultMemoryAddressTransformer, MemoryAddress, UnknownMemoryAddressPerspective,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type UnknownMemoryAddress<T = DefaultMemoryAddressTransformer> =
    MemoryAddress<UnknownMemoryAddressPerspective, T>;
