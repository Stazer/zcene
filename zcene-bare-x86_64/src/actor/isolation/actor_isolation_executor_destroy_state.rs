use crate::actor::{
    ActorIsolationEnvironment, ActorIsolationExecutorContext, ActorIsolationExecutorState,
    ActorIsolationExecutorStateHandler, ActorRootEnvironment,
};
use alloc::boxed::Box;
use core::arch::asm;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use zcene_bare::memory::{LeakingAllocator, LeakingBox};
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorMessageChannel, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorIsolationExecutorDestroyState<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    actor: Box<AI>,
    context: Option<ActorIsolationExecutorContext>,
    #[Constructor(default)]
    marker: PhantomData<(AR, H)>,
}

impl<AI, AR, H> ActorIsolationExecutorStateHandler<AI, AR, H>
    for ActorIsolationExecutorDestroyState<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    fn context(&self) -> Option<&ActorIsolationExecutorContext> {
        self.context.as_ref()
    }

    fn actor_mut(&mut self) -> &mut Box<AI> {
        &mut self.actor
    }

    extern "C" fn main(actor: *mut AI) -> ! {
        let actor = unsafe { LeakingBox::from_raw_in(actor, LeakingAllocator) };

        let mut future_context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.destroy(()));

        let result = match pinned.as_mut().poll(&mut future_context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    fn preemption_state(
        self,
        context: ActorIsolationExecutorContext,
    ) -> ActorIsolationExecutorState<AI, AR, H> {
        Self::new(self.actor, Some(context)).into()
    }

    fn next_state(self) -> Option<ActorIsolationExecutorState<AI, AR, H>> {
        None
    }
}
