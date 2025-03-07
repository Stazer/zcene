use crate::memory::address::MemoryAddressTransformer;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct IdentityMemoryAddressTransformer;

impl MemoryAddressTransformer for IdentityMemoryAddressTransformer {
    fn transform(value: usize) -> usize {
        value
    }
}
