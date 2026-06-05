use crate::actor::{
    ActorRootEnvironmentCreateContext, ActorRootEnvironmentDestroyContext,
    ActorRootEnvironmentHandleContext,
};
use crate::common::time::Timer;
use crate::platform::x86_64::X86_64Platform;
use crate::platform::Platform;
use core::fmt::Write;
use zcene_core::actor::{
    Actor, ActorEnterError, ActorEnvironmentAllocator, ActorMessage, ActorMessageChannelAddress,
};
use zcene_core::actor::{
    ActorEnvironment, ActorEnvironmentReference, ActorEnvironmentSpawn, ActorMessageChannel,
    ActorSpawnError,
};
use zcene_core::future::runtime::{FutureRuntime, FutureRuntimeHandler, FutureRuntimeReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootEnvironment<H = crate::future::runtime::FutureRuntimeHandler>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
    platform: X86_64Platform,
}

impl<H> ActorEnvironment for ActorRootEnvironment<H>
where
    H: FutureRuntimeHandler,
{
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext<'a> = ActorRootEnvironmentCreateContext<'a, H>;
    type HandleContext<'a, M>
        = ActorRootEnvironmentHandleContext<'a, H, M>
    where
        M: ActorMessage;
    type DestroyContext<'a> = ActorRootEnvironmentDestroyContext<'a, H>;
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

impl<A, H> ActorEnvironmentSpawn<A> for ActorRootEnvironment<H>
where
    A: Actor<Self>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        self: &ActorEnvironmentReference<Self>,
        mut actor: A,
    ) -> Result<<ActorRootEnvironment<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        self.future_runtime.spawn({
            let environment = self.clone();

            async move {
                // TODO: Handle result
                let _result = actor
                    .create(ActorRootEnvironmentCreateContext::new(&*environment))
                    .await;

                while let Some(message) = receiver.receive().await {
                    // TODO: Handle result
                    let _result = actor
                        .handle(ActorRootEnvironmentHandleContext::new(
                            &*environment,
                            message,
                        ))
                        .await;
                }

                // TODO: Handle result
                let _result = actor
                    .destroy(ActorRootEnvironmentDestroyContext::new(&*environment))
                    .await;
            }
        })?;

        Ok(<ActorRootEnvironment<H> as ActorEnvironment>::Address::<A>::new(sender))
    }
}

impl<H> ActorRootEnvironment<H>
where
    H: FutureRuntimeHandler<Allocator = alloc::alloc::Global>,
{
    pub fn root<F>(
        handler: F,
        boot_info: &'static mut bootloader_api::BootInfo,
    ) -> ActorEnvironmentReference<Self>
    where
        F: FnOnce() -> H,
    {
        let platform = X86_64Platform::initialize(boot_info);

        let runtime = FutureRuntime::new(handler()).unwrap();

        ActorEnvironmentReference::new(Self::new(runtime, platform))
    }

    pub fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }

    pub fn logger(&self) -> impl Write {
        self.platform.logger().writer()
    }

    pub fn timer(&self) -> &impl Timer {
        self.platform.timer()
    }
}

impl ActorRootEnvironment<crate::future::runtime::FutureRuntimeHandler> {
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
