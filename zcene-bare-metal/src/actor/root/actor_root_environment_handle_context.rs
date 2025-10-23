use crate::actor::ActorRootEnvironment;
use zcene_core::future::runtime::{FutureRuntimeHandler};
use zcene_core::actor::{ActorMessage};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentHandleContext<'a, H, M>
where
    H: FutureRuntimeHandler,
    M: ActorMessage,
{
    environment: &'a ActorRootEnvironment<H>,
    message: M,
}
