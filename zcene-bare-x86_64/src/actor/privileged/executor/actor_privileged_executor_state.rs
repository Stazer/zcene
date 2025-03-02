use crate::actor::{
    ActorPrivilegedExecutorCreateState, ActorPrivilegedExecutorDestroyState,
    ActorPrivilegedExecutorHandleState, ActorPrivilegedExecutorReceiveState,
};
use zcene_core::actor::{Actor, ActorEnvironment};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorPrivilegedExecutorState<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    Create(ActorPrivilegedExecutorCreateState<A, E>),
    Receive(ActorPrivilegedExecutorReceiveState<A, E>),
    Handle(ActorPrivilegedExecutorHandleState<A, E>),
    Destroy(ActorPrivilegedExecutorDestroyState<A, E>),
}
