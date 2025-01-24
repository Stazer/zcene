use crate::actor::{
    Actor, ActorAddressReference, ActorEnterError, ActorHandler, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorSpawnError,
    ActorCommonHandleContext,
};
use crate::future::runtime::{
    FutureRuntimeHandler, FutureRuntimeReference,
};
use core::marker::PhantomData;
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

    type Allocator = H::Allocator;

    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(&self, mut actor: A) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        self.future_runtime.spawn(async move {
            actor.create(()).await;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                actor
                    .handle(Self::HandleContext::<A::Message>::new(message))
                    .await;
            }

            actor.destroy(()).await;
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}
