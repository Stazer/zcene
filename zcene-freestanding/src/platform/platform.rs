pub trait Platform: Sized {
    type BootInfo;

    fn initialize(boot_info: Self::BootInfo) -> Self;
}
