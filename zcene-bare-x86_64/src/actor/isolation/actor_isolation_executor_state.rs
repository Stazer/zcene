use crate::actor::{
    ActorIsolationEnvironment, ActorIsolationExecutorCreateState,
    ActorIsolationExecutorDestroyState, ActorIsolationExecutorHandleState,
    ActorIsolationExecutorReceiveState, ActorRootEnvironment,
};
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorMessageChannel, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, From};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(unnamed)]
pub enum ActorIsolationExecutorState<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    Create(ActorIsolationExecutorCreateState<AI, AR, H>),
    Receive(ActorIsolationExecutorReceiveState<AI, AR, H>),
    Handle(ActorIsolationExecutorHandleState<AI, AR, H>),
    Destroy(ActorIsolationExecutorDestroyState<AI, AR, H>),
}
