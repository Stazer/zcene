#![feature(precise_capturing_in_traits)]

#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use zcene_bare_metal::actor::ActorRootEnvironment;
use zcene_bare_metal::define_system;
use zcene_bare_metal::kernel::logger::println;
use zcene_core::actor::{Actor, ActorCreateError, ActorEnvironment, ActorFuture};
use zcene_core::future::runtime::FutureRuntimeHandler;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct EntryPointActor;

impl<H> Actor<ActorRootEnvironment<H>> for EntryPointActor
where
    H: FutureRuntimeHandler,
{
    type Message = ();

    async fn create(
        &mut self,
        context: <ActorRootEnvironment<H> as ActorEnvironment>::CreateContext,
    ) -> Result<(), ActorCreateError> {
        println!("Hello World");

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(EntryPointActor::default());
