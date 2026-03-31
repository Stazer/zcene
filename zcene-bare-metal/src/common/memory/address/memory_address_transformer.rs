pub trait MemoryAddressTransformer {
    fn transform(value: usize) -> usize;
}
