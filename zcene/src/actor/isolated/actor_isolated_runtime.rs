use crate::actor::isolated::{ActorIsolatedAllocator, ActorIsolatedCell, ActorIsolatedEnvironment};
use crate::actor::unprivileged::ActorUnprivilegedSystemCalls;
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use crate::actor::ActorIsolatedPhase;
use zcene_core::actor::{Actor, ActorCommonHandleContext};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedRuntime;

impl ActorIsolatedRuntime {
    /// Runs one phase of the actor and ends it. This is what the entry point of an isolated
    /// program calls; see `define_isolated_actor!`.
    pub fn run<A, F>(
        allocator: &ActorIsolatedAllocator,
        cell: &ActorIsolatedCell<A>,
        phase: u64,
        message: *const A::Message,
        construct: F,
    ) -> !
    where
        A: Actor<ActorIsolatedEnvironment>,
        F: FnOnce() -> A,
    {
        if phase == ActorIsolatedPhase::Create as u64 {
            allocator.initialize();

            // SAFETY: the create phase runs exactly once, before anything else.
            let actor = unsafe { cell.store(construct()) };

            let _ = Self::block_on(actor.create(()));
        } else if phase == ActorIsolatedPhase::Handle as u64 {
            // SAFETY: the kernel hands over a valid message for this phase, and the actor was
            // stored by the create phase.
            let (actor, message) = unsafe { (cell.get(), (*message).clone()) };

            let _ = Self::block_on(actor.handle(ActorCommonHandleContext::new(message)));
        } else if phase == ActorIsolatedPhase::Destroy as u64 {
            // SAFETY: the actor was stored by the create phase and is not used afterwards.
            unsafe {
                let _ = Self::block_on(cell.get().destroy(()));
                cell.drop();
            }
        } else {
            ActorUnprivilegedSystemCalls::fault();
        }

        ActorUnprivilegedSystemCalls::exit()
    }

    fn block_on<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => ActorUnprivilegedSystemCalls::yield_now(),
            }
        }
    }
}
