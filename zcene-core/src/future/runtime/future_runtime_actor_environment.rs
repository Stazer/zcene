use crate::actor::{
    Actor, ActorCommonHandleContext, ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator,
    ActorEnvironmentEnterable, ActorEnvironmentSpawnable, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorSpawnError,
};
use crate::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct FutureRuntimeActorEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
}

impl<H> ActorEnvironment for FutureRuntimeActorEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();
}

impl<H> ActorEnvironmentAllocator for FutureRuntimeActorEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Allocator = H::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

impl<H> ActorEnvironmentEnterable<FutureRuntimeActorEnvironment<H>> for ()
where
    H: FutureRuntimeHandler,
{
    fn enter(self, environment: &FutureRuntimeActorEnvironment<H>) -> Result<(), ActorEnterError> {
        environment.future_runtime.run();

        Ok(())
    }
}

impl<A, H> ActorEnvironmentSpawnable<A, FutureRuntimeActorEnvironment<H>> for A
where
    A: Actor<FutureRuntimeActorEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        mut self,
        environment: &FutureRuntimeActorEnvironment<H>,
    ) -> Result<<FutureRuntimeActorEnvironment<H> as ActorEnvironment>::Address<A>, ActorSpawnError>
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let address = <FutureRuntimeActorEnvironment<H> as ActorEnvironment>::Address::new(sender);

        environment.future_runtime.spawn(async move {
            // TODO: Handle result
            let _result = self.create(()).await;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                // TODO: Handle result
                let _result = self
                    .handle(
                        <FutureRuntimeActorEnvironment<H> as ActorEnvironment>::HandleContext::<
                            A::Message,
                        >::new(message),
                    )
                    .await;
            }

            // TODO: Handle result
            let _result = self.destroy(()).await;
        })?;

        Ok(address)
    }
}
