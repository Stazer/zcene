use crate::memory::address::MemoryAddressPerspective;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug)]
pub struct UnknownMemoryAddressPerspective;

impl MemoryAddressPerspective for UnknownMemoryAddressPerspective {}
