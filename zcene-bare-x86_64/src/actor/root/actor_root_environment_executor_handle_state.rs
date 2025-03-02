use crate::actor::ActorRootEnvironment;
use core::marker::PhantomData;
use zcene_core::actor::Actor;
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorRootEnvironmentExecutorHandleState<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    actor: A,
    message: A::Message,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
