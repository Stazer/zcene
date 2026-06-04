use crate::actor::ActorIsolationExecutorDeadlinePreemptionContext;
use ztd::{Constructor, Inner, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Clone, Debug, Inner, Method)]
#[Method(accessors)]
pub struct ActorIsolationExecutorDeadlinePreemptionEvent {
    context: ActorIsolationExecutorDeadlinePreemptionContext,
}
