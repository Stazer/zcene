#![feature(sync_unsafe_cell)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::fmt::Write;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use zcene::actor::ActorRootEnvironment;
use zcene::core::actor::{
    Actor, ActorCreateError, ActorEnvironment, ActorHandleError, ActorMessage, ActorMessageSender,
};
use zcene::define_system;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct PingMessage(u32);

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct RootActor;

impl Actor<ActorRootEnvironment> for RootActor {
    type Message = PingMessage;

    async fn create<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let _ = write!(context.environment().logger(), "hello world\n");

        Ok(())
    }

    async fn handle<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> Result<(), ActorHandleError> {
        let _ = write!(
            context.environment().logger(),
            "root: received ping {}\n",
            context.message().0,
        );

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Spawns `RootActor` and then keeps nudging it with messages from a second actor, so the
/// message path is actually exercised end to end (and, with multiple cores, exercised across
/// cores) instead of the system sitting idle after printing its one startup line.
#[derive(Default)]
pub struct PingActor;

impl Actor<ActorRootEnvironment> for PingActor {
    type Message = ();

    async fn create<'a>(
        &'a mut self,
        _context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        // `_context.environment()` only gives a plain `&ActorRootEnvironment`, but `spawn`
        // needs the `Arc`-wrapped reference; the global accessor is the way to get that from
        // inside an actor (it's valid as soon as any actor is running at all).
        let root = ActorRootEnvironment::get()
            .spawn(RootActor::default())
            .unwrap();

        for counter in 0u32.. {
            spin_delay();

            if root.send(PingMessage(counter)).await.is_err() {
                break;
            }
        }

        Ok(())
    }
}

/// No timer is guaranteed to be real time this early in boot (see `TimerService`'s software
/// fallback), so this is a plain cycle-counted pause rather than an actual sleep.
fn spin_delay() {
    for _ in 0..20_000_000u64 {
        core::hint::spin_loop();
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(PingActor::default());
