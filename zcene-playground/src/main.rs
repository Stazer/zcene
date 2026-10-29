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

#[cfg(target_os = "none")]
use zcene::core::actor::ActorEnvironmentAllocator;
#[cfg(target_os = "none")]
use zcene::driver::ps2::keyboard::KeyboardActor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
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

        // The PS/2 driver is x86_64-specific, so there is no `KeyboardActor` to spawn on the
        // hosted platform.
        #[cfg(target_os = "none")]
        {
            let keyboard = KeyboardActor::new(ActorRootEnvironment::get().allocator().clone());
            let _keyboard = ActorRootEnvironment::get().spawn(keyboard).unwrap();
        }

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

/// Looks up `RootActor`'s recipient via `ActorDiscovery` (keyed by its message type, `PingMessage`)
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
        let recipients = context.environment().discovery().lookup::<PingMessage>();

        let _ = write!(
            context.environment().logger(),
            "lookup: found {} recipient(s) for PingMessage\n",
            recipients.len(),
        );

        for recipient in recipients.iter() {
            let _ = recipient.send(PingMessage(999)).await;
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

        #[cfg(target_os = "none")]
        let unprivileged = unprivileged::spawn(&root);

        let _lookup = ActorRootEnvironment::get()
            .spawn(LookupActor::default())
            .unwrap();

        for counter in 0u32.. {
            spin_delay();

            if root.send(PingMessage(counter)).await.is_err() {
                break;
            }

            #[cfg(target_os = "none")]
            if counter % 3 == 0 {
                let _ = unprivileged.send(PingMessage(counter)).await;
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

/// An actor running in ring 3. It cannot log (port I/O and `cli` fault outside ring 0), so it
/// proves it ran by forwarding every ping to `RootActor` with an offset, via a system call.
#[cfg(target_os = "none")]
mod unprivileged {
    use super::{PingMessage, RootActor};
    use core::num::NonZero;
    use zcene::actor::{
        ActorRootEnvironment, ActorUnprivilegedAddress, ActorUnprivilegedEnvironment,
        ActorUnprivilegedKernelPerspective, ActorUnprivilegedRecipient,
    };
    use zcene::core::actor::{
        Actor, ActorCreateError, ActorEnvironment, ActorHandleError, ActorMessageChannelAddress,
        ActorMessageSender,
    };

    pub const CREATED: u32 = 1_000_000_000;
    pub const FORWARDED: u32 = 2_000_000_000;

    pub struct UnprivilegedActor {
        // Granted by the handlers passed to `spawn`: `RootActor`.
        root: ActorUnprivilegedRecipient<PingMessage>,
    }

    impl Actor<ActorUnprivilegedEnvironment> for UnprivilegedActor {
        type Message = PingMessage;

        async fn create<'a>(
            &'a mut self,
            _context: <ActorUnprivilegedEnvironment as ActorEnvironment>::CreateContext<'a>,
        ) -> Result<(), ActorCreateError> {
            let _ = self.root.send(PingMessage(CREATED)).await;
            Ok(())
        }

        async fn handle<'a>(
            &'a mut self,
            context: <ActorUnprivilegedEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
        ) -> Result<(), ActorHandleError> {
            // Long enough to overrun the deadline a few times, so preemption is exercised.
            for _ in 0..3_000_000u64 {
                core::hint::spin_loop();
            }

            let _ = self.root.send(PingMessage(FORWARDED + context.message().0)).await;
            Ok(())
        }
    }

    pub fn spawn(
        root: &ActorMessageChannelAddress<RootActor, ActorRootEnvironment>,
    ) -> ActorUnprivilegedAddress<UnprivilegedActor, ActorUnprivilegedKernelPerspective> {
        let environment = ActorRootEnvironment::get().unprivileged_environment();

        let mut handlers = environment.handlers();
        let root = handlers.grant_address(root).unwrap();

        environment
            .spawn(UnprivilegedActor { root }, NonZero::new(5), handlers)
            .unwrap()
    }
}

#[cfg(target_os = "none")]
extern crate alloc;

////////////////////////////////////////////////////////////////////////////////////////////////////

define_system!(PingActor::default());
