use crate::actor::{
    ActorRootEnvironmentCreateContext, ActorRootEnvironmentDestroyContext,
    ActorRootEnvironmentExecutor, ActorRootEnvironmentHandleContext,
};
use crate::common::time::Timer;
use crate::platform::x86_64::X86_64Platform;
use crate::platform::Platform;
use alloc::sync::Arc;
use core::fmt::Write;
use zcene_core::actor::{
    Actor, ActorEnterError, ActorEnvironmentAllocator, ActorMessage, ActorMessageChannelAddress,
};
use zcene_core::actor::{
    ActorEnvironment, ActorEnvironmentReference, ActorMessageChannel, ActorSpawnError,
};
use zcene_core::future::runtime::{FutureRuntime, FutureRuntimeHandler, FutureRuntimeReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootEnvironment<H = crate::future::runtime::FutureRuntimeHandler, P = crate::platform::x86_64::X86_64Platform>
where
    H: FutureRuntimeHandler,
    P: Platform,
{
    future_runtime: FutureRuntimeReference<H>,
    platform: P,
}

impl<H, P> ActorEnvironment for ActorRootEnvironment<H, P>
where
    H: FutureRuntimeHandler,
    P: Platform,
{
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext<'a> = ActorRootEnvironmentCreateContext<'a, H, P>;
    type HandleContext<'a, M>
        = ActorRootEnvironmentHandleContext<'a, H, P, M>
    where
        M: ActorMessage;
    type DestroyContext<'a> = ActorRootEnvironmentDestroyContext<'a, H, P>;
}

impl<H, P> ActorEnvironmentAllocator for ActorRootEnvironment<H, P>
where
    H: FutureRuntimeHandler,
    P: Platform,
{
    type Allocator = <H as FutureRuntimeHandler>::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

impl<H, P> ActorRootEnvironment<H, P>
where
    H: FutureRuntimeHandler,
    P: Platform,
{
    pub fn spawn<A>(
        self: &ActorEnvironmentReference<Self>,
        actor: A,
    ) -> Result<<ActorRootEnvironment<H, P> as ActorEnvironment>::Address<A>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) =
            ActorMessageChannel::<A::Message>::new_unbounded_in(self.allocator().clone());

        self.future_runtime.spawn(
            ActorRootEnvironmentExecutor::new(self.clone(), actor, receiver),
        )?;

        Ok(<ActorRootEnvironment<H, P> as ActorEnvironment>::Address::<A>::new(sender))
    }

    pub fn platform(&self) -> &P {
        &self.platform
    }

    pub fn future_runtime(&self) -> &FutureRuntimeReference<H> {
        &self.future_runtime
    }
}

impl<H, P> ActorRootEnvironment<H, P>
where
    H: FutureRuntimeHandler,
    P: Platform<Allocator = <H as FutureRuntimeHandler>::Allocator>,
{
    pub fn root<F>(
        handler: F,
        boot_info: P::BootInfo,
    ) -> ActorEnvironmentReference<Self>
    where
        F: FnOnce(P::Allocator) -> H,
    {
        let platform = P::initialize(boot_info);
        let allocator = platform.allocator();

        let runtime = FutureRuntime::new(handler(allocator.clone())).unwrap();

        Arc::new_in(Self::new(runtime, platform), allocator)
    }

    pub fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

impl<H> ActorRootEnvironment<H, X86_64Platform>
where
    H: FutureRuntimeHandler,
{
    pub fn logger(&self) -> impl Write {
        self.platform.logger().writer()
    }

    pub fn timer(&self) -> &impl Timer {
        self.platform.timer()
    }
}

impl ActorRootEnvironment<crate::future::runtime::FutureRuntimeHandler, X86_64Platform> {
    pub fn get() -> &'static ActorEnvironmentReference<Self> {
        unsafe {
            crate::ACTOR_ROOT_ENVIRONMENT
                .get()
                .as_ref()
                .unwrap()
                .as_ptr()
                .as_ref()
                .unwrap()
        }
    }
}
