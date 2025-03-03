use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct ActorIsolationExecutorSystemCallContext {
    rsp: u64,
    rip: u64,
    rflags: u64,
}
