#![feature(allocator_ext)]

use std::alloc::Global;
use std::error::Error;
use zcene_core::actor::{ActorEnvironmentEnterable, ActorEnvironmentReference};
use zcene_core::future::runtime::FutureRuntime;
use zcene_hosted::actor::ActorRootEnvironment;
use zcene_hosted::future::runtime::FutureRuntimeHandler;

////////////////////////////////////////////////////////////////////////////////////////////////////

fn main() -> Result<(), Box<dyn Error>> {
    let environment = ActorEnvironmentReference::new_in(
        ActorRootEnvironment::new(FutureRuntime::new(FutureRuntimeHandler::default())?),
        Global,
    );

    ().enter(&environment)?;

    Ok(())
}
