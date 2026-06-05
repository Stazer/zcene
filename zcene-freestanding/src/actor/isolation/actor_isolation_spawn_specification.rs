use crate::actor::{
    ActorIsolationEnvironment, ActorIsolationExecutor, ActorIsolationMessageHandler,
    ActorRootEnvironment,
};
use core::alloc::boxed::Box;
use core::alloc::vec::Vec;
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorEnvironmentAllocator, ActorEnvironmentSpawnable,
    ActorMessageChannel, ActorSpawnError, ActorSystemReference,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorIsolationSpawnSpecification<AI, AR, H, P>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: crate::platform::Platform,
{
    actor: AI,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    message_handlers: Vec<
        Box<
            dyn ActorIsolationMessageHandler<ActorRootEnvironment<H, P>>,
            <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
        >,
        <ActorRootEnvironment<H, P> as ActorEnvironmentAllocator>::Allocator,
    >,
    #[Constructor(default)]
    marker: PhantomData<(AR, H, P)>,
}

impl<AI, AR, H, P> ActorEnvironmentSpawnable<AR, ActorRootEnvironment<H, P>>
    for ActorIsolationSpawnSpecification<AI, AR, H, P>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H, P>, Message = AI::Message>,
    H: FutureRuntimeHandler,
    P: crate::platform::Platform,
{
    fn spawn(
        self,
        system: &ActorSystemReference<ActorRootEnvironment<H, P>>,
    ) -> Result<<ActorRootEnvironment<H, P> as ActorEnvironment>::Address<AR>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<AR::Message>::new_unbounded();

        system.environment().future_runtime().spawn(
            ActorIsolationExecutor::<AI, AR, H, P>::new(
                system.allocator().clone(),
                Box::new(self.actor),
                receiver,
                self.deadline_in_milliseconds,
                self.message_handlers,
            )
            .run(),
        )?;

        Ok(<ActorRootEnvironment<H, P> as ActorEnvironment>::Address::new(
            sender,
        ))
    }
}
