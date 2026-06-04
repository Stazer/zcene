use crate::actor::{
    ActorIsolationExecutorDeadlinePreemptionContext, ActorIsolationExecutorSystemCallContext,
};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, From)]
#[From(unnamed)]
pub enum ActorIsolationExecutorContext {
    SystemCall(ActorIsolationExecutorSystemCallContext),
    DeadlinePreemption(ActorIsolationExecutorDeadlinePreemptionContext),
}
