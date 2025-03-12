use crate::actor::{ActorRootEnvironment, ActorRootEnvironmentExecutor};
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorEnvironmentSpawnable,
    ActorMessageChannel, ActorSpawnError, ActorSystemReference,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootSpawnSpecification<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    actor: A,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, H> ActorEnvironmentSpawnable<A, ActorRootEnvironment<H>>
    for ActorRootSpawnSpecification<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        self,
        system: &ActorSystemReference<ActorRootEnvironment<H>>,
    ) -> Result<<ActorRootEnvironment<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        system.environment().future_runtime().spawn(
            ActorRootEnvironmentExecutor::new(system.clone(), self.actor, receiver, None).run(),
        )?;

        Ok(<ActorRootEnvironment<H> as ActorEnvironment>::Address::new(
            sender,
        ))
    }
}
