extern "C" {
    pub static SMP_SECTIONS_START: usize;
    pub static SMP_SECTIONS_END: usize;
    pub static SMP_SECTIONS_SIZE: usize;

    pub fn smp_real_mode_entry();
}
