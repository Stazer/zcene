use crate::actor::{ActorRootEnvironment, ActorRootEnvironmentExecutor};
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
        environment: &ActorRootEnvironment<H>,
    ) -> Result<<ActorRootEnvironment<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        environment.future_runtime().spawn(
            /*async move {
                self.actor.create(()).await;

                loop {
                    let message = match receiver.receive().await {
                        Some(message) => message,
                        None => break,
                    };

                    self.actor.handle(zcene_core::actor::ActorCommonHandleContext::new(message)).await;
                }

                self.actor.destroy(()).await;
            }*/
            ActorRootEnvironmentExecutor::new(
                //Some(ActorRootEnvironmentExecutorCreateState::new(self.actor).into()),
                self.actor,
                receiver,
                ActorCommonContextBuilder::default(),
                None,
            )
            .run(),
        )?;

        Ok(<ActorRootEnvironment<H> as ActorEnvironment>::Address::new(
            sender,
        ))
    }
}
