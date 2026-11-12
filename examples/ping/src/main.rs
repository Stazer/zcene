#![feature(sync_unsafe_cell)]
#![no_std]
#![no_main]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use core::num::NonZero;
use zcene::actor::{
    ActorUnprivilegedEnvironmentPrivilegedPerspective, ActorUnprivilegedRecipient, ActorRootEnvironment,
};
use zcene::core::actor::{
    Actor, ActorCreateError, ActorEnvironment, ActorHandleError, ActorMessageChannelAddress,
    ActorMessageSender,
};
use zcene::{define_system, include_actor_unprivileged_image};

////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct RootActor;

impl Actor<ActorRootEnvironment> for RootActor {
    type Message = u32;

    async fn handle<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = write!(
            context.environment().logger(),
            "root: received {}\n",
            context.message(),
        );

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////

type Program = ActorUnprivilegedRecipient<u32, ActorUnprivilegedEnvironmentPrivilegedPerspective>;

/// Starts `image` in an address space of its own, with `root` as its recipient 0.
macro_rules! spawn_program {
    ($root:expr, $name:literal) => {{
        let environment = ActorRootEnvironment::get().unprivileged_environment();

        let mut handlers = environment.handlers();
        let _root = handlers.grant_address($root).unwrap();

        let image = include_actor_unprivileged_image!(concat!(env!("EXAMPLE_IMAGES"), "/", $name))
            .unwrap();

        environment.spawn::<u32>(image, NonZero::new(5), handlers).unwrap()
    }};
}

////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct StartActor;

impl Actor<ActorRootEnvironment> for StartActor {
    type Message = ();

    async fn create<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let _ = write!(context.environment().logger(), "start: spawning programs\n");

        let root: ActorMessageChannelAddress<RootActor, ActorRootEnvironment> =
            ActorRootEnvironment::get().spawn(RootActor::default()).unwrap();

        // Two different programs, and two instances of the same one: every instance counts
        // in memory of its own.
        let counter: Program = spawn_program!(&root, "counter");
        let counter_again: Program = spawn_program!(&root, "counter");
        let doubler: Program = spawn_program!(&root, "doubler");

        for message in 1..=3 {
            // `counter` answers with 1_000_000 + how many messages it has seen.
            let _ = counter.send(message).await;
            // `doubler` answers with 2_000_000 + twice the message.
            let _ = doubler.send(message).await;
        }

        let _ = counter_again.send(0).await;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(StartActor::default());
