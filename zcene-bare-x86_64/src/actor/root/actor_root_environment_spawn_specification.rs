use crate::actor::{
    ActorRootEnvironment, ActorRootEnvironmentExecutor, ActorRootEnvironmentExecutorCreateState,
};
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{
    Actor, ActorCommonContextBuilder, ActorEnvironment, ActorEnvironmentSpawnable,
    ActorMessageChannel, ActorSpawnError,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootEnvironmentSpawnSpecification<A, H>
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
    for ActorRootEnvironmentSpawnSpecification<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        self,
        environment: &ActorRootEnvironment<H>,
    ) -> Result<<ActorRootEnvironment<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        environment
            .future_runtime()
            .spawn(ActorRootEnvironmentExecutor::new(
                Some(ActorRootEnvironmentExecutorCreateState::new(self.actor).into()),
                receiver,
                ActorCommonContextBuilder::default(),
                None,
            ))?;

        Ok(<ActorRootEnvironment<H> as ActorEnvironment>::Address::new(
            sender,
        ))
    }
}
