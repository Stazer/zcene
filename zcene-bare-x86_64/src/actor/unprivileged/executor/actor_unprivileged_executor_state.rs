use crate::actor::{
    ActorUnprivilegedExecutorCreateState, ActorUnprivilegedExecutorDestroyState,
    ActorUnprivilegedExecutorHandleState, ActorUnprivilegedExecutorReceiveState,
    ActorUnprivilegedHandler,
};
use zcene_core::actor::{Actor, ActorEnvironment};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorUnprivilegedExecutorState<A, E>
where
    A: Actor<E> + Actor<ActorUnprivilegedHandler>,
    E: ActorEnvironment,
{
    Create(ActorUnprivilegedExecutorCreateState<A, E>),
    Receive(ActorUnprivilegedExecutorReceiveState<A, E>),
    Handle(ActorUnprivilegedExecutorHandleState<A, E>),
    Destroy(ActorUnprivilegedExecutorDestroyState<A, E>),
}
