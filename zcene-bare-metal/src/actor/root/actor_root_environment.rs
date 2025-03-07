use zcene_core::actor::{
    Actor, ActorCommonHandleContext, ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator,
    ActorEnvironmentEnterable, ActorMessage, ActorMessageChannelAddress,
};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::{Constructor, Method};

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

    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();
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
    fn enter(self, environment: &ActorRootEnvironment<H>) -> Result<(), ActorEnterError> {
        environment.future_runtime.run();

        Ok(())
    }
}
