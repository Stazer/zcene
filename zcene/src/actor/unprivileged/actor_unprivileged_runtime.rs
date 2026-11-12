use crate::actor::unprivileged::ActorUnprivilegedSystemCalls;
use crate::actor::unprivileged::{ActorUnprivilegedAllocator, ActorUnprivilegedCell, ActorUnprivilegedEnvironment};
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use crate::actor::ActorUnprivilegedPhase;
use zcene_core::actor::{Actor, ActorCommonHandleContext};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedRuntime;

impl ActorUnprivilegedRuntime {
    /// Runs one phase of the actor and ends it. This is what the entry point of an unprivileged
    /// program calls; see `define_unprivileged_actor!`.
    pub fn run<A, F>(
        allocator: &ActorUnprivilegedAllocator,
        cell: &ActorUnprivilegedCell<A>,
        phase: u64,
        message: *const A::Message,
        construct: F,
    ) -> !
    where
        A: Actor<ActorUnprivilegedEnvironment>,
        F: FnOnce() -> A,
    {
        if phase == ActorUnprivilegedPhase::Create as u64 {
            allocator.initialize();

            // SAFETY: the create phase runs exactly once, before anything else.
            let actor = unsafe { cell.store(construct()) };

            let _ = Self::block_on(actor.create(()));
        } else if phase == ActorUnprivilegedPhase::Handle as u64 {
            // SAFETY: the kernel hands over a valid message for this phase, and the actor was
            // stored by the create phase.
            let (actor, message) = unsafe { (cell.get(), (*message).clone()) };

            let _ = Self::block_on(actor.handle(ActorCommonHandleContext::new(message)));
        } else if phase == ActorUnprivilegedPhase::Destroy as u64 {
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
