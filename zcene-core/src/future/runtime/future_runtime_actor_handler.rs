use crate::actor::{
    Actor, ActorAddressReference, ActorContextMessageProvider, ActorEnterError, ActorHandler,
    ActorMessage, ActorSpawnError,
};
use crate::future::runtime::{
    FutureRuntimeActorAddress, FutureRuntimeHandler, FutureRuntimeReference, FutureRuntimeActorHandleContext,
};
use async_channel::unbounded;
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
        = FutureRuntimeActorAddress<A, Self>
    where
        A: Actor<Self>;

    type Allocator = H::Allocator;

    type CreateContext = ();
    type HandleContext<M>
        = FutureRuntimeActorHandleContext<M>
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
        let (sender, receiver) = unbounded::<A::Message>();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        self.future_runtime.spawn(async move {
            actor.create(()).await;

            loop {
                let message = match receiver.recv().await {
                    Ok(message) => message,
                    Err(_) => break,
                };

                actor.handle(Self::HandleContext::<A::Message>::new(message)).await;
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
