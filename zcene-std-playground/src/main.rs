use std::error::Error;
use zcene_core::actor::ActorSystem;
use zcene_core::future::runtime::FutureRuntime;
use zcene_std::actor::ActorRootEnvironment;
use zcene_std::future::runtime::FutureRuntimeHandler;

////////////////////////////////////////////////////////////////////////////////////////////////////

fn main() -> Result<(), Box<dyn Error>> {
    let actor_system = ActorSystem::try_new(ActorRootEnvironment::new(FutureRuntime::new(FutureRuntimeHandler::default())?))?;

    actor_system.enter()?;

    Ok(())
}
