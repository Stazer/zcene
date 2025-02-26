use ztd::From;

use core::task::Poll;
use ztd::{Constructor, Inner, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, From)]
#[From(unnamed)]
pub enum ActorUnprivilegedStageExecutorEvent {
    None,
    SystemCall(ActorUnprivilegedStageExecutorSystemCall),
    DeadlinePreemption(ActorUnprivilegedStageExecutorDeadlinePreemption),
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

#[derive(Constructor, Debug, Inner)]
pub struct ActorUnprivilegedStageExecutorDeadlinePreemption {
    context: ActorUnprivilegedStageExecutorDeadlinePreemptionContext,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Clone, Debug, Default, Method)]
#[Method(accessors)]
#[repr(C, packed)]
pub struct ActorUnprivilegedStageExecutorDeadlinePreemptionContext {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
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
