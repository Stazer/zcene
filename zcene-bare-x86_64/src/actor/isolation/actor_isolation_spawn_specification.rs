use crate::actor::{
    ActorIsolationEnvironment, ActorIsolationExecutor, ActorIsolationExecutorCreateState,
    ActorIsolationMessageHandler, ActorRootEnvironment,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{
    Actor, ActorCommonContextBuilder, ActorEnvironment, ActorEnvironmentAllocator,
    ActorEnvironmentSpawnable, ActorMessageChannel, ActorSpawnError,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorIsolationSpawnSpecification<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>, Message = AI::Message>,
    H: FutureRuntimeHandler,
{
    actor: AI,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    message_handlers: Vec<
        Box<
            //dyn ActorIsolationMessageHandler<ActorRootEnvironment<H>>,
            //
            dyn ActorIsolationMessageHandler,
            //<ActorRootEnvironment<H> as ActorEnvironmentAllocator>::Allocator,
        >,
        <ActorRootEnvironment<H> as ActorEnvironmentAllocator>::Allocator,
    >,
    #[Constructor(default)]
    marker: PhantomData<(AR, H)>,
}

impl<AI, AR, H> ActorEnvironmentSpawnable<AR, ActorRootEnvironment<H>>
    for ActorIsolationSpawnSpecification<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>, Message = AI::Message>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        self,
        environment: &ActorRootEnvironment<H>,
    ) -> Result<<ActorRootEnvironment<H> as ActorEnvironment>::Address<AR>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<AR::Message>::new_unbounded();

        environment
            .future_runtime()
            .spawn(ActorIsolationExecutor::<AI, AR, H>::new(
                environment.allocator().clone(),
                Some(ActorIsolationExecutorCreateState::new(Box::new(self.actor), None).into()),
                receiver,
                self.deadline_in_milliseconds,
                self.message_handlers,
            ))?;

        Ok(<ActorRootEnvironment<H> as ActorEnvironment>::Address::new(
            sender,
        ))
    }
}
