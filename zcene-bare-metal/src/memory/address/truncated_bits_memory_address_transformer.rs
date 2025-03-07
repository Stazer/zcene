use crate::memory::address::MemoryAddressTransformer;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct TruncatedBitsMemoryAddressTransformer<const N: usize>;

impl<const N: usize> MemoryAddressTransformer for TruncatedBitsMemoryAddressTransformer<N> {
    fn transform(value: usize) -> usize {
        value % (1 << N)
    }
}
