use zcene_core::actor::{
    Actor, ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator, ActorEnvironmentEnterable,
    ActorEnvironmentReference, ActorEnvironmentSpawnable, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorSpawnError,
};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentCreateContext<H>
where
    H: FutureRuntimeHandler,
{
    system: ActorEnvironmentReference<ActorRootEnvironment<H>>,
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentHandleContext<H, M>
where
    H: FutureRuntimeHandler,
{
    system: ActorEnvironmentReference<ActorRootEnvironment<H>>,
    message: M,
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentDestroyContext<H>
where
    H: FutureRuntimeHandler,
{
    system: ActorEnvironmentReference<ActorRootEnvironment<H>>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
}

impl<H> ActorEnvironment for ActorRootEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext<'a> = ActorRootEnvironmentCreateContext<H>;
    type HandleContext<'a, M>
        = ActorRootEnvironmentHandleContext<H, M>
    where
        M: ActorMessage;
    type DestroyContext<'a> = ActorRootEnvironmentDestroyContext<H>;
}

impl<H> ActorEnvironmentAllocator for ActorRootEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Allocator = <H as FutureRuntimeHandler>::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

impl<H> ActorEnvironmentEnterable<ActorRootEnvironment<H>> for ()
where
    H: FutureRuntimeHandler,
{
    fn enter(
        self,
        system: &ActorEnvironmentReference<ActorRootEnvironment<H>>,
    ) -> Result<(), ActorEnterError> {
        system.future_runtime.run();

        Ok(())
    }
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorSpawnSpecification<A> {
    actor: A,
}

impl<A, H> ActorEnvironmentSpawnable<A, ActorRootEnvironment<H>> for ActorSpawnSpecification<A>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        mut self,
        system: &ActorEnvironmentReference<ActorRootEnvironment<H>>,
    ) -> Result<<ActorRootEnvironment<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, mut receiver) =
            ActorMessageChannel::<A::Message>::new_unbounded_in(system.allocator().clone());

        let inner_system = system.clone();

        system.future_runtime().spawn(async move {
            let system = inner_system;

            // TODO
            let _result = self
                .actor
                .create(ActorRootEnvironmentCreateContext::new(system.clone()))
                .await;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                let _result = self
                    .actor
                    .handle(ActorRootEnvironmentHandleContext::new(
                        system.clone(),
                        message,
                    ))
                    .await;
            }

            // TODO
            let _result = self
                .actor
                .destroy(ActorRootEnvironmentDestroyContext::new(system))
                .await;
        })?;

        Ok(<ActorRootEnvironment<H> as ActorEnvironment>::Address::new(
            sender,
        ))
    }
}
