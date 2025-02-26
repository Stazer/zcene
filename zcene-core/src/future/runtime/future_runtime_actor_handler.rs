use crate::actor::{
    ActorAllocatorHandler, Actor, ActorAddressReference, ActorCommonHandleContext, ActorEnterError, ActorHandler,
    ActorMessage, ActorSpawnHandler, ActorMessageChannel, ActorMessageChannelAddress, ActorSpawnError, ActorEnterHandler,
};
use crate::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct FutureRuntimeActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
}

impl<H> ActorHandler for FutureRuntimeActorHandler<H>
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

impl<H> ActorAllocatorHandler for FutureRuntimeActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type Allocator = H::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

impl<H> ActorEnterHandler for FutureRuntimeActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type EnterSpecification = ();

    fn enter(&self, _specification: Self::EnterSpecification) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

impl<H> ActorSpawnHandler for FutureRuntimeActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type SpawnSpecification<A>
        = A
    where
        A: Actor<Self>;

    fn spawn<A>(
        &self,
        mut actor: Self::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender),
            self.allocator().clone(),
        )?;

        self.future_runtime.spawn(async move {
            // TODO: Handle result
            actor.create(()).await;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                // TODO: Handle result
                actor
                    .handle(Self::HandleContext::<A::Message>::new(message))
                    .await;
            }

            // TODO: Handle result
            actor.destroy(()).await;
        });

        Ok(reference)
    }
}
