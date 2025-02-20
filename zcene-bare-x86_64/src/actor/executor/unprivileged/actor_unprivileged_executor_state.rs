use crate::actor::{
    ActorUnprivilegedExecutorCreateState, ActorUnprivilegedExecutorDestroyState,
    ActorUnprivilegedExecutorHandleState, ActorUnprivilegedExecutorReceiveState,
};
use zcene_core::actor::{Actor, ActorHandler};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorUnprivilegedExecutorState<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    Create(ActorUnprivilegedExecutorCreateState<A, H>),
    Receive(ActorUnprivilegedExecutorReceiveState<A, H>),
    Handle(ActorUnprivilegedExecutorHandleState<A, H>),
    Destroy(ActorUnprivilegedExecutorDestroyState<A, H>),
}
