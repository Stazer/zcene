#[derive(Clone, Debug)]
pub enum KernelConfigurationParameter {
    Absolute(usize),
    Relative(usize, usize),
}

#[derive(Clone, Debug)]
pub struct KernelConfiguration {
    heap_start: PhysicalMemoryAddress,
    heap_size: KernelConfigurationParameter,
}
