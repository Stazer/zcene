#[derive(Clone, Debug)]
pub enum KernelConfigurationParameter {
    Absolute(usize),
    Relative(usize, usize),
}

pub struct KernelConfiguration {
    heap_size: KernelConfigurationParameter,
}
