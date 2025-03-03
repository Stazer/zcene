use crate::actor::{
    ActorIsolationEnvironment, ActorIsolationExecutorContext, ActorIsolationExecutorEvent,
    ActorIsolationExecutorState, ActorRootEnvironment,
};
use alloc::boxed::Box;
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorMessageChannel, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorIsolationExecutorStateHandler<AI, AR, H>: Sized
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    fn context(&self) -> Option<&ActorIsolationExecutorContext>;

    fn actor_mut(&mut self) -> &mut Box<AI>;

    extern "C" fn main(actor: *mut AI) -> !;

    fn preemption_state(
        self,
        context: ActorIsolationExecutorContext,
    ) -> ActorIsolationExecutorState<AI, AR, H>;

    fn next_state(self) -> Option<ActorIsolationExecutorState<AI, AR, H>>;
}
