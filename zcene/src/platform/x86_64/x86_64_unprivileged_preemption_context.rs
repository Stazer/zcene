#[derive(Clone)]
#[repr(C, align(16))]
pub struct X86_64FpuState([u8; 512]);

impl X86_64FpuState {
    pub const SIZE: usize = 512;
}

impl Default for X86_64FpuState {
    fn default() -> Self {
        Self([0; Self::SIZE])
    }
}

#[derive(Clone, Default)]
#[repr(C)]
pub struct X86_64UnprivilegedPreemptionContext {
    pub fpu: X86_64FpuState,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

const _: () = assert!(core::mem::offset_of!(X86_64UnprivilegedPreemptionContext, fpu) == 0);
const _: () = assert!(
    core::mem::offset_of!(X86_64UnprivilegedPreemptionContext, rax) == X86_64FpuState::SIZE
);
