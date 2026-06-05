pub trait Platform: Sized + Send + Sync + 'static {
    type BootInfo;

    fn initialize(boot_info: Self::BootInfo) -> Self;
}
