use core::marker::PhantomData;
use zcene_core::actor;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorEnterError, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorSpawnError,
};
use zcene_core::future::runtime::{
    FutureRuntimeActorHandleContext, FutureRuntimeHandler, FutureRuntimeReference,
};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

use alloc::sync::Arc;

#[derive(Default)]
pub struct Shared {

}

#[derive(Constructor)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
    //shared: Arc<Shared>
}

impl<H> actor::ActorHandler for ActorHandler<H>
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
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        //let shared = self.shared.clone();

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

use crate::kernel::Kernel;
use core::fmt::Write;

use x86_64::structures::idt::InterruptStackFrame;

impl<H> ActorHandler<H>
where
    H: FutureRuntimeHandler,

{
    pub fn reschedule(&self, stack_frame: InterruptStackFrame) {
        Kernel::get().logger().writer(|w| write!(w, "{:?}", stack_frame));
    }
}
