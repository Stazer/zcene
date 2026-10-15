use crate::actor::ActorRootEnvironment;
use crate::platform::Platform;
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentDestroyContext<'a, H, P>
where
    H: FutureRuntimeHandler,
    P: Platform,
{
    environment: &'a ActorRootEnvironment<H, P>,
}
