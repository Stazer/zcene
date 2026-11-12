pub struct UnprivilegedAddressSpaceSpecification {
    pub size: usize,
    pub executable_length: usize,
    pub required_base: Option<u64>,
}

pub trait UnprivilegedAddressSpace: Send + Sync + 'static {
    fn user_base(&self) -> u64;
    fn kernel_base(&self) -> *mut u8;
    fn size(&self) -> usize;
}
