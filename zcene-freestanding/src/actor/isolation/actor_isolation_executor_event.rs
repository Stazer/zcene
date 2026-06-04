use crate::actor::{
    ActorIsolationExecutorDeadlinePreemptionEvent, ActorIsolationExecutorSystemCallEvent,
};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, From)]
#[From(unnamed)]
pub enum ActorIsolationExecutorEvent {
    SystemCall(ActorIsolationExecutorSystemCallEvent),
    DeadlinePreemption(ActorIsolationExecutorDeadlinePreemptionEvent),
    Exception,
}
