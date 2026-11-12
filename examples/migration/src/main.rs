#![feature(sync_unsafe_cell)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use zcene::actor::ActorRootEnvironment;
use zcene::core::actor::{
    Actor, ActorCreateError, ActorEnvironment, ActorHandleError, ActorMessageSender,
};
use zcene::define_system;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
struct GreetMessage(u32);

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct GreeterV1;

impl Actor<ActorRootEnvironment> for GreeterV1 {
    type Message = GreetMessage;

    async fn handle<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = write!(
            context.environment().logger(),
            "v1: hello #{}\n",
            context.message().0,
        );

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct GreeterV2;

impl Actor<ActorRootEnvironment> for GreeterV2 {
    type Message = GreetMessage;

    async fn handle<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = write!(
            context.environment().logger(),
            "v2: HELLO #{}!!\n",
            context.message().0,
        );

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct RootActor;

impl Actor<ActorRootEnvironment> for RootActor {
    type Message = ();

    async fn create<'a>(
        &'a mut self,
        _context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let greeter = ActorRootEnvironment::get()
            .spawn(GreeterV1::default())
            .unwrap();

        for counter in 0..3u32 {
            let _ = greeter.send(GreetMessage(counter)).await;
        }

        let _ = greeter.migrate(GreeterV2::default()).await;

        for counter in 3..6u32 {
            let _ = greeter.send(GreetMessage(counter)).await;
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(RootActor::default());
