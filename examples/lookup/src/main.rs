#![feature(sync_unsafe_cell)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(target_os = "none", no_std)]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use zcene::actor::ActorRootEnvironment;
use zcene::core::actor::{
    Actor, ActorCreateError, ActorDiscovery, ActorEnvironment, ActorHandleError,
    ActorMessageSender,
};
use zcene::define_system;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
struct GreetMessage(u32);

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct GreeterActor;

impl Actor<ActorRootEnvironment> for GreeterActor {
    type Message = GreetMessage;

    async fn handle<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = write!(
            context.environment().logger(),
            "greeter: hello #{}\n",
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
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let _greeter = ActorRootEnvironment::get()
            .spawn(GreeterActor::default())
            .unwrap();

        let recipients = context.environment().discovery().lookup::<GreetMessage>();

        let _ = write!(
            context.environment().logger(),
            "lookup: found {} recipient(es) for GreetMessage\n",
            recipients.len(),
        );

        for recipient in recipients.iter() {
            let _ = recipient.send(GreetMessage(42)).await;
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(RootActor::default());
