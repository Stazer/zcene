use zcene_core::actor::{
    Actor, ActorCommonHandleContext, ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator,
    ActorEnvironmentEnterable, ActorMessage, ActorMessageChannelAddress, ActorSystemReference,
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
    system: ActorSystemReference<ActorRootEnvironment<H>>,
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentCreateContext2<'a, H>
where
    H: FutureRuntimeHandler,
{
    system: &'a ActorSystemReference<ActorRootEnvironment<H>>,
}


#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorRootEnvironmentDestroyContext<H>
where
    H: FutureRuntimeHandler,
{
    system: ActorSystemReference<ActorRootEnvironment<H>>,
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

    type CreateContext = ActorRootEnvironmentCreateContext<H>;
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
    fn enter(
        self,
        system: &ActorSystemReference<ActorRootEnvironment<H>>,
    ) -> Result<(), ActorEnterError> {
        system.environment().future_runtime.run();

        Ok(())
    }
}
