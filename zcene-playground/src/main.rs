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
    Actor, ActorCreateError, ActorDiscovery, ActorEnvironment, ActorHandleError, ActorMessage,
    ActorMessageSender,
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

/// Looks up `RootActor`'s mailbox via `ActorDiscovery` (keyed by its message type, `PingMessage`)
/// instead of being handed it directly, to exercise `ActorRootEnvironment`'s automatic
/// registration on `spawn`.
#[derive(Default)]
pub struct LookupActor;

impl Actor<ActorRootEnvironment> for LookupActor {
    type Message = ();

    async fn create<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::CreateContext<'a>,
    ) -> Result<(), ActorCreateError> {
        let mailboxes = context.environment().discovery().lookup::<PingMessage>();

        let _ = write!(
            context.environment().logger(),
            "lookup: found {} mailbox(es) for PingMessage\n",
            mailboxes.len(),
        );

        for mailbox in mailboxes.iter() {
            let _ = mailbox.send(PingMessage(999)).await;
        }

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

        let _lookup = ActorRootEnvironment::get()
            .spawn(LookupActor::default())
            .unwrap();

        for counter in 0u32.. {
            spin_delay();

            if root.send(PingMessage(counter)).await.is_err() {
                break;
            }

            // The runtime is cooperative: an unbounded channel's `send` never actually returns
            // `Pending`, so without this, a single core never gets back to the executor's queue
            // to run anything else (including `RootActor` itself) — fine with a core to spare,
            // but it starves the whole system on `-smp 1`.
            yield_now().await;
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

/// Returns `Pending` once (rescheduling this task via the waker) before resolving, so the
/// calling core gets back to the executor's queue in between.
fn yield_now() -> impl Future<Output = ()> {
    struct YieldNow(bool);

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                context.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    YieldNow(false)
}

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(PingActor::default());
