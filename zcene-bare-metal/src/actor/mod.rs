mod isolation;
mod root;

pub use isolation::*;
pub use root::*;

use zcene_core::actor::{
    self,
    ActorEnvironmentAllocator,
    ActorEnvironmentEnterable,
    ActorSystemReference,
    ActorEnterError,
    ActorMessage,
    ActorMessageChannelAddress,
    Actor,
};

use ztd::{Constructor, Method};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorCreateContext<H>
where
    H: FutureRuntimeHandler,
{
    system: ActorSystemReference<ActorRootEnvironment<H>>,
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorHandleContext<H, M>
where
    H: FutureRuntimeHandler,
    M: ActorMessage,
{
    system: ActorSystemReference<ActorEnvironment<H>>,
    message: M,
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorDestroyContext<H>
where
    H: FutureRuntimeHandler,
{
    system: ActorSystemReference<ActorEnvironment<H>>,
}

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
}

impl<H> actor::ActorEnvironment for ActorEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext = ActorCreateContext<H>;
    type HandleContext<M>
        = ActorHandleContext<H, M>
    where
        M: ActorMessage;
    type DestroyContext = ActorDestroyContext<H>;
}

impl<H> ActorEnvironmentAllocator for ActorEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Allocator = <H as FutureRuntimeHandler>::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

impl<H> ActorEnvironmentEnterable<ActorEnvironment<H>> for ()
where
    H: FutureRuntimeHandler,
{
    fn enter(
        self,
        system: &ActorSystemReference<ActorEnvironment<H>>,
    ) -> Result<(), ActorEnterError> {
        system.environment().future_runtime.run();

        Ok(())
    }
}
