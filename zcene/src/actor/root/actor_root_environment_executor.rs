use crate::actor::{
    ActorRootEnvironment, ActorRootEnvironmentCreateContext, ActorRootEnvironmentDestroyContext,
    ActorRootEnvironmentHandleContext,
};
use crate::platform::Platform;
use alloc::boxed::Box;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use zcene_core::actor::{
    Actor, ActorEnvironmentAllocator, ActorEnvironmentReference, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorRootEnvironmentExecutor<A, H, P>
where
    H: FutureRuntimeHandler,
{
    future: Pin<
        Box<
            dyn Future<Output = ()> + Send + 'static,
            <H as FutureRuntimeHandler>::Allocator,
        >,
    >,
    _marker: PhantomData<fn() -> (A, H, P)>,
}

impl<A, H, P> ActorRootEnvironmentExecutor<A, H, P>
where
    A: Actor<ActorRootEnvironment<H, P>> + Send + 'static,
    A::Message: Send + 'static,
    H: FutureRuntimeHandler,
    P: Platform,
    ActorEnvironmentReference<ActorRootEnvironment<H, P>>: Send + 'static,
    ActorMessageChannelReceiver<A::Message, <H as FutureRuntimeHandler>::Allocator>: Send + 'static,
{
    pub fn new(
        environment: ActorEnvironmentReference<ActorRootEnvironment<H, P>>,
        actor: A,
        receiver: ActorMessageChannelReceiver<A::Message, <H as FutureRuntimeHandler>::Allocator>,
    ) -> Self {
        let allocator = environment.allocator().clone();

        Self {
            future: Box::pin_in(async move {
                let mut actor = actor;
                let mut receiver = receiver;

                let _result = actor
                    .create(ActorRootEnvironmentCreateContext::new(&*environment))
                    .await;

                while let Some(message) = receiver.receive().await {
                    let _result = actor
                        .handle(ActorRootEnvironmentHandleContext::new(&*environment, message))
                        .await;
                }

                let _result = actor
                    .destroy(ActorRootEnvironmentDestroyContext::new(&*environment))
                    .await;
            }, allocator),
            _marker: PhantomData,
        }
    }
}

impl<A, H, P> Future for ActorRootEnvironmentExecutor<A, H, P>
where
    H: FutureRuntimeHandler,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
