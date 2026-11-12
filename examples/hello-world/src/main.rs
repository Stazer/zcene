#![feature(sync_unsafe_cell)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", no_std)]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use zcene::actor::ActorRootEnvironment;
use zcene::core::actor::{Actor, ActorCreateError, ActorEnvironment};
use zcene::define_system;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct RootActor;

impl Actor<ActorRootEnvironment> for RootActor {
    type Message = ();

    async fn create<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let _ = write!(context.environment().logger(), "hello world\n");

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(RootActor::default());
