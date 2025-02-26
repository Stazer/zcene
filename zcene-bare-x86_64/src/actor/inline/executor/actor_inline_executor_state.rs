use crate::actor::{
    ActorInlineExecutorCreateState, ActorInlineExecutorDestroyState,
    ActorInlineExecutorHandleState, ActorInlineExecutorReceiveState,
};
use zcene_core::actor::{Actor, ActorHandler};
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorInlineExecutorState<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    Create(ActorInlineExecutorCreateState<A, H>),
    Receive(ActorInlineExecutorReceiveState<A, H>),
    Handle(ActorInlineExecutorHandleState<A, H>),
    Destroy(ActorInlineExecutorDestroyState<A, H>),
}
