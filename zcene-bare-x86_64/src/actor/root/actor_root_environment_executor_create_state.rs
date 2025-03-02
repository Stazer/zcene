use crate::actor::ActorRootEnvironment;
use core::marker::PhantomData;
use zcene_core::actor::Actor;
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorRootEnvironmentExecutorCreateState<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    actor: A,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
