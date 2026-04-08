use ztd::{Constructor};
use zcene_core::actor::{ActorEnvironment, ActorEnvironmentReference, ActorEnvironmentSpawn, ActorMessageChannel, ActorEnvironmentSpawnable, ActorSpawnError};
use crate::actor::ActorRootEnvironmentArchitectureService;
use zcene_core::future::runtime::{FutureRuntime, FutureRuntimeHandler, FutureRuntimeReference};
use zcene_core::actor::{
    ActorEnvironmentAllocator,
    ActorEnterError,
    ActorMessage,
    ActorMessageChannelAddress,
    Actor,
};
use crate::actor::{
    ActorRootEnvironmentCreateContext,
    ActorRootEnvironmentHandleContext,
    ActorRootEnvironmentDestroyContext,
    ActorRootEnvironmentLoggerService,
    ActorRootEnvironmentTimerService,
    ActorRootEnvironmentMemoryService,
};
use crate::common::time::Timer;
use core::fmt::Write;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootEnvironment<H = crate::future::runtime::FutureRuntimeHandler>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
    architecture_service: ActorRootEnvironmentArchitectureService,
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

        self.future_runtime.spawn(
            {
                let environment = self.clone();

                async move {
                    // TODO: Handle result
                    let _result = actor
                        .create(ActorRootEnvironmentCreateContext::new(&*environment))
                        .await;

                    while let Some(message) = receiver.receive().await {
                        // TODO: Handle result
                        let _result = actor
                            .handle(ActorRootEnvironmentHandleContext::new(&*environment, message))
                            .await;
                    }

                    // TODO: Handle result
                    let _result = actor
                        .destroy(ActorRootEnvironmentDestroyContext::new(&*environment))
                        .await;
                }
            }
        )?;

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
        let architecture_service = ActorRootEnvironmentArchitectureService::new(boot_info);

        let runtime = FutureRuntime::new(
            handler(),
        ).unwrap();

        ActorEnvironmentReference::new(
            Self::new(
                runtime,
                architecture_service,
            ),
        )
    }

    pub fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }

    pub fn logger(&self) -> impl Write {
        self.architecture_service.logger().writer()
    }

    pub fn timer(&self) -> &impl Timer {
        self.architecture_service.timer()
    }
}

impl ActorRootEnvironment<crate::future::runtime::FutureRuntimeHandler> {
    pub fn get() -> &'static ActorEnvironmentReference<Self> {
        unsafe { crate::ACTOR_ROOT_ENVIRONMENT.get().as_ref().unwrap().as_ptr().as_ref().unwrap() }
    }
}
