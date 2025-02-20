use crate::actor::{
    ActorPrivilegedExecutorCreateState, ActorPrivilegedExecutorDestroyState,
    ActorPrivilegedExecutorHandleState, ActorPrivilegedExecutorReceiveState,
};
use zcene_core::actor::{Actor, ActorHandler};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorPrivilegedExecutorState<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    Create(ActorPrivilegedExecutorCreateState<A, H>),
    Receive(ActorPrivilegedExecutorReceiveState<A, H>),
    Handle(ActorPrivilegedExecutorHandleState<A, H>),
    Destroy(ActorPrivilegedExecutorDestroyState<A, H>),
}
