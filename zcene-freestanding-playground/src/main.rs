#![feature(sync_unsafe_cell)]
#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use zcene_core::actor::{Actor, ActorCreateError, ActorEnvironment, ActorMessage};
use zcene_freestanding::actor::ActorRootEnvironment;
use zcene_freestanding::define_system;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct RootActor;

impl Actor<ActorRootEnvironment> for RootActor {
    type Message = ();

    async fn create<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let _ = write!(context.environment().logger(), "hello world");

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(RootActor::default());
