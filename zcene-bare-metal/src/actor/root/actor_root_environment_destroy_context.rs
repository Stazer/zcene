use crate::actor::ActorRootEnvironment;
use zcene_core::future::runtime::{FutureRuntimeHandler};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentDestroyContext<'a, H>
where
    H: FutureRuntimeHandler,
{
    environment: &'a ActorRootEnvironment<H>,
}
