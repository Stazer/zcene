use crate::actor::ActorRootEnvironment;
use crate::platform::Platform;
use zcene_core::actor::ActorMessage;
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentHandleContext<'a, H, P, M>
where
    H: FutureRuntimeHandler,
    P: Platform,
    M: ActorMessage,
{
    environment: &'a ActorRootEnvironment<H, P>,
    message: M,
}
