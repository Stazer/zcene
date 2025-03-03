use crate::actor::{ActorIsolationExecutorSystemCallContext, ActorIsolationExecutorSystemCallType};
use ztd::{Constructor, Inner, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug, Inner, Method)]
#[Method(accessors)]
pub struct ActorIsolationExecutorSystemCallEvent {
    r#type: ActorIsolationExecutorSystemCallType,
    context: ActorIsolationExecutorSystemCallContext,
}
