use ztd::From;

use core::task::Poll;
use ztd::{Constructor, Inner, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, From)]
#[From(unnamed)]
pub enum ActorUnprivilegedStageExecutorEvent {
    None,
    SystemCall(ActorUnprivilegedStageExecutorSystemCall),
    DeadlinePreemption,
    Exception,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, From)]
#[From(unnamed)]
pub enum ActorUnprivilegedStageExecutorContext {
    SystemCall(ActorUnprivilegedStageExecutorSystemCallContext),
    DeadlinePreemption(ActorUnprivilegedStageExecutorDeadlinePreemptionContext),
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct ActorUnprivilegedStageExecutorSystemCallContext {
    rsp: u64,
    rip: u64,
    rflags: u64,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method)]
#[Method(accessors)]
pub struct ActorUnprivilegedStageExecutorDeadlinePreemptionContext {
    rsp: u64,
    rip: u64,
    rflags: u64,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Method, Inner)]
#[Method(accessors)]
pub struct ActorUnprivilegedStageExecutorSystemCall {
    r#type: ActorUnprivilegedStageExecutorSystemCallType,
    context: ActorUnprivilegedStageExecutorSystemCallContext,
}

#[derive(Debug)]
pub enum ActorUnprivilegedStageExecutorSystemCallType {
    Continue,
    Preempt,
    Poll(Poll<()>),
    Unknown(usize),
}
