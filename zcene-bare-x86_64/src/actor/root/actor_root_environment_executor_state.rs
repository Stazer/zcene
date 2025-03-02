use crate::actor::ActorRootEnvironment;
use crate::actor::{
    ActorRootEnvironmentExecutorCreateState, ActorRootEnvironmentExecutorDestroyState,
    ActorRootEnvironmentExecutorHandleState, ActorRootEnvironmentExecutorReceiveState,
};
use zcene_core::actor::Actor;
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(From)]
#[From(all)]
pub enum ActorRootEnvironmentExecutorState<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    Create(ActorRootEnvironmentExecutorCreateState<A, H>),
    Receive(ActorRootEnvironmentExecutorReceiveState<A, H>),
    Handle(ActorRootEnvironmentExecutorHandleState<A, H>),
    Destroy(ActorRootEnvironmentExecutorDestroyState<A, H>),
}
