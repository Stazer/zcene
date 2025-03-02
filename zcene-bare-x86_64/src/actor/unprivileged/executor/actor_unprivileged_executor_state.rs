use crate::actor::{
    ActorUnprivilegedExecutorCreateState, ActorUnprivilegedExecutorDestroyState,
    ActorUnprivilegedExecutorHandleState, ActorUnprivilegedExecutorReceiveState,
    ActorUnprivilegedHandler,
};
use zcene_core::actor::{Actor, ActorEnvironment, ActorMessage};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorUnprivilegedExecutorState<A, E, M>
where
    A: Actor<ActorUnprivilegedHandler>,
    E: ActorEnvironment,
    M: ActorMessage,
{
    Create(ActorUnprivilegedExecutorCreateState<A, E>),
    Receive(ActorUnprivilegedExecutorReceiveState<A, E>),
    Handle(ActorUnprivilegedExecutorHandleState<A, E, M>),
    Destroy(ActorUnprivilegedExecutorDestroyState<A, E>),
}
