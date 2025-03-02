use crate::actor::{
    ActorPrivilegedExecutor, ActorPrivilegedExecutorCreateState, ActorUnprivilegedExecutor,
    ActorUnprivilegedExecutorCreateState,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::ActorCommonBounds;
use zcene_core::actor::ActorEnterError;
use zcene_core::actor::ActorEnvironmentEnterable;
use zcene_core::actor::ActorEnvironmentSpawnable;
use zcene_core::actor::{
    self, Actor, ActorAddressReference, ActorCommonContextBuilder, ActorCommonHandleContext,
    ActorEnvironmentAllocator, ActorMailbox, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorSpawnError,
};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(accessors)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
}

impl<H> actor::ActorEnvironment for ActorHandler<H>
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

impl<H> ActorEnvironmentAllocator for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type Allocator = <H as FutureRuntimeHandler>::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

use zcene_core::actor::ActorEnvironment;

impl<H> ActorEnvironmentEnterable<ActorHandler<H>> for ()
where
    H: FutureRuntimeHandler,
{
    fn enter(self, environment: &ActorHandler<H>) -> Result<(), ActorEnterError> {
        environment.future_runtime.run();

        Ok(())
    }
}

#[derive(Constructor)]
pub struct ActorPrivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorEnvironment,
{
    actor: A,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, H> ActorEnvironmentSpawnable<A, ActorHandler<H>>
    for ActorPrivilegedHandlerSpawnSpecification<A, ActorHandler<H>>
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        self,
        environment: &ActorHandler<H>,
    ) -> Result<<ActorHandler<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        environment
            .future_runtime
            .spawn(ActorPrivilegedExecutor::new(
                Some(ActorPrivilegedExecutorCreateState::new(self.actor).into()),
                receiver,
                ActorCommonContextBuilder::default(),
                None,
            ))?;

        Ok(<ActorHandler<H> as ActorEnvironment>::Address::new(sender))
    }
}

pub trait ActorEnvironmentTransformer<E>: Sized
where
    E: ActorEnvironment,
{
    type Output: Actor<E>;

    fn transform(self) -> Self::Output;
}

#[derive(Constructor)]
pub struct ActorUnprivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorEnvironment,
{
    actor: A,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

use crate::actor::ActorUnprivilegedHandler;

impl<A, H> ActorEnvironmentSpawnable<A, ActorHandler<H>>
    for ActorUnprivilegedHandlerSpawnSpecification<A, ActorHandler<H>>
where
    A: Actor<ActorHandler<H>> + ActorEnvironmentTransformer<ActorUnprivilegedHandler>,
    H: FutureRuntimeHandler,
{
    fn spawn(
        self,
        environment: &ActorHandler<H>,
    ) -> Result<<ActorHandler<H> as ActorEnvironment>::Address<A>, ActorSpawnError> {
        let (sender, receiver) =
            ActorMessageChannel::<<A as Actor<ActorHandler<H>>>::Message>::new_unbounded();

        let actor = self.actor.transform();

        environment
            .future_runtime
            .spawn(ActorUnprivilegedExecutor::new(
                Some(
                    ActorUnprivilegedExecutorCreateState::<_, ActorHandler<H>>::new(
                        Box::new(actor),
                        None,
                    )
                    .into(),
                ),
                receiver,
                None,
            ))?;

        Ok(<ActorHandler<H> as ActorEnvironment>::Address::new(sender))
    }
}

use zcene_core::actor::ActorMessageSender;
