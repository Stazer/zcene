#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use zcene_bare_metal::actor::{ActorRootEnvironment};
use zcene_bare_metal::define_system;
use zcene_core::actor::{ActorMessage, ActorContext, ActorEnvironment, Actor, ActorCreateError, ActorFuture};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct RootActor;

#[derive(Clone)]
pub enum RootActorMessage {}

impl Actor<ActorRootEnvironment> for RootActor {
    type Message = RootActorMessage;

    async fn create<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let _ = write!(
            context.environment().logger(),
            "Hello World",
        );

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(RootActor::default());
