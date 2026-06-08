use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Constructor, Method)]
#[Method(accessors)]
pub struct X86_64UnprivilegedSystemCallContext {
    rsp: u64,
    rip: u64,
    rflags: u64,
}
