//! The playground comes in two halves in this one file, built as two binaries (see `Cargo.toml`):
//!
//! - `zcene-playground`: the system itself, `mod kernel`.
//! - `zcene-playground-isolated`: a Rust program that the system starts in an isolated address
//!   space, `mod program`. It is built with `--features isolated-program` and embedded into the
//!   first one, so it has to be built first (`cargo make` takes care of that).

#![cfg_attr(not(feature = "isolated-program"), feature(sync_unsafe_cell))]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(all(feature = "isolated-program", target_os = "none"))]
extern crate alloc;

#[cfg(all(feature = "isolated-program", not(target_os = "none")))]
fn main() {}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(not(feature = "isolated-program"))]
mod kernel {
    ////////////////////////////////////////////////////////////////////////////////////////////////////

    use core::fmt::Write;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use zcene::actor::{ActorIsolatedMessage, ActorRootEnvironment};
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

    #[derive(Clone, Copy, Debug)]
    #[repr(transparent)]
    pub struct PingMessage(u32);

    unsafe impl ActorIsolatedMessage for PingMessage {}

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

            // Two instances of the same program: each counts in memory of its own.
            #[cfg(target_os = "none")]
            let probes = (isolated::spawn(&root), isolated::spawn(&root));

            // One instance each for the things that must go wrong without taking anything else down.
            #[cfg(target_os = "none")]
            for command in [isolated::FAULT, isolated::PANIC, isolated::FPU] {
                let probe = isolated::spawn(&root);
                let _ = probe.send(PingMessage(command)).await;
            }

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
                    let _ = probes.0.send(PingMessage(counter)).await;
                    let _ = probes.1.send(PingMessage(counter)).await;
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
            ActorUnprivilegedEnvironmentPrivilegedPerspective, ActorUnprivilegedRecipient,
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
        ) -> ActorUnprivilegedAddress<UnprivilegedActor, ActorUnprivilegedEnvironmentPrivilegedPerspective> {
            let environment = ActorRootEnvironment::get().unprivileged_environment();

            let mut handlers = environment.handlers();
            let root = handlers.grant_address(root).unwrap();

            environment
                .spawn(UnprivilegedActor { root }, NonZero::new(5), handlers)
                .unwrap()
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Starts the Rust program from `src/bin/zcene-playground-isolated.rs` in an isolated address
    /// space of its own. It has to be built first (`cargo make` takes care of that).
    #[cfg(target_os = "none")]
    mod isolated {
        use super::{PingMessage, RootActor};
        use core::num::NonZero;
        use zcene::actor::{
            ActorIsolatedEnvironmentPrivilegedPerspective, ActorIsolatedRecipient, ActorRootEnvironment,
        };
        use zcene::core::actor::ActorMessageChannelAddress;

        // Commands the program understands; every other value is a ping. The same values as in the
        // program.
        pub const FAULT: u32 = 0xFFFF_0001;
        pub const PANIC: u32 = 0xFFFF_0002;
        pub const FPU: u32 = 0xFFFF_0003;

        pub type Probe =
            ActorIsolatedRecipient<PingMessage, ActorIsolatedEnvironmentPrivilegedPerspective>;

        pub fn spawn(root: &ActorMessageChannelAddress<RootActor, ActorRootEnvironment>) -> Probe {
            let environment = ActorRootEnvironment::get().isolated_environment();

            // The program's only recipient (index 0) is `RootActor`.
            let mut handlers = environment.handlers();
            let _root = handlers.grant_address(root).unwrap();

            environment
                .spawn::<PingMessage>(
                    zcene::include_actor_isolated_image!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../target/x86_64/release/zcene-playground-isolated"
                    ))
                    .unwrap(),
                    NonZero::new(5),
                    handlers,
                )
                .unwrap()
        }
    }
}

#[cfg(not(feature = "isolated-program"))]
zcene::define_system!(kernel::PingActor::default());

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A Rust program that runs in an isolated address space, started by the kernel playground
/// (`src/main.rs`, module `isolated`). It answers on recipient 0, the `RootActor`.
///
/// The messages are plain `u32`s, the same bytes as the kernel's `PingMessage`. Three special
/// values are commands, everything else is a ping that is counted in the program's own memory
/// (so two instances never see each other's count) and answered with 3500000000 + count.
#[cfg(feature = "isolated-program")]
mod program {
    use alloc::format;
    use alloc::vec::Vec;
    use core::arch::asm;
    use zcene::actor::{ActorIsolatedEnvironment, ActorIsolatedRecipient};
    use zcene::core::actor::{Actor, ActorEnvironment, ActorHandleError, ActorMessageSender};
    use zcene::define_isolated_actor;

    // The same values as in the kernel playground.
    const FAULT: u32 = 0xFFFF_0001;
    const PANIC: u32 = 0xFFFF_0002;
    const FPU: u32 = 0xFFFF_0003;

    pub struct ProbeActor {
        count: u32,
        root: ActorIsolatedRecipient<u32>,
    }

    impl Actor<ActorIsolatedEnvironment> for ProbeActor {
        type Message = u32;

        async fn handle<'a>(
            &'a mut self,
            context: <ActorIsolatedEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
        ) -> Result<(), ActorHandleError> {
            let reply = match *context.message() {
                // The kernel image is not mapped in this address space: a page fault, which ends
                // this actor and nothing else.
                FAULT => {
                    let _ = unsafe { core::ptr::read_volatile(0x20_0000 as *const u8) };
                    0
                }

                // A panic ends in an invalid instruction, which the kernel reports as a fault.
                PANIC => panic!("asked to panic"),

                // 0xE0000001 if the SSE registers survived being preempted, 0xE0000000 if not.
                FPU => 0xE000_0000 | u32::from(sse_registers_survive_preemption()),

                _ => {
                    self.count += 1;

                    // Exercises the heap and `format!` (which needs `dyn Write`).
                    let mut numbers = Vec::new();
                    numbers.extend(0..self.count);
                    assert_eq!(format!("{}", numbers.len()).parse::<u32>().ok(), Some(self.count));

                    3_500_000_000 + numbers.len() as u32
                }
            };

            let _ = self.root.send(reply).await;

            Ok(())
        }
    }

    /// Fills the low half of every XMM register with a different pattern, spins long enough to
    /// be preempted several times and checks that the patterns are still there. All in one
    /// `asm!` block, so the compiler cannot touch the registers in between.
    fn sse_registers_survive_preemption() -> bool {
        let difference: u64;

        unsafe {
            asm!(
                ".irp index, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15",
                "mov rax, (\\index + 1) * 0x0101010101010101",
                "movq xmm\\index, rax",
                ".endr",
                "mov rcx, 300000000",
                "2:",
                "dec rcx",
                "jnz 2b",
                "xor edx, edx",
                ".irp index, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15",
                "movq rax, xmm\\index",
                "mov rcx, (\\index + 1) * 0x0101010101010101",
                "xor rax, rcx",
                "or rdx, rax",
                ".endr",
                out("rax") _,
                out("rcx") _,
                out("rdx") difference,
                out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
                out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
                out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
                out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
            );
        }

        difference == 0
    }

    define_isolated_actor!(
        ProbeActor,
        ProbeActor {
            count: 0,
            root: ActorIsolatedRecipient::new(0),
        }
    );
}
