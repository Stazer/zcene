use crate::actor::{
    ActorPrivilegedExecutor, ActorPrivilegedExecutorCreateState, ActorSpawnSpecification,
    ActorSpawnSpecificationInner, ActorSpawnSpecificationType, ActorUnprivilegedExecutor,
    ActorUnprivilegedExecutorCreateState,
};
use alloc::boxed::Box;
use zcene_core::actor::{
    self, Actor, ActorAddressReference, ActorAllocatorHandler, ActorCommonContextBuilder,
    ActorCommonHandleContext, ActorDiscoverHandler, ActorEnterError, ActorEnterHandler,
    ActorMailbox, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress, ActorSpawnError,
    ActorSpawnHandler,
};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
}

impl<H> actor::ActorHandler for ActorHandler<H>
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

impl<H> ActorAllocatorHandler for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type Allocator = <H as FutureRuntimeHandler>::Allocator;

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }
}

impl<H> ActorEnterHandler for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type EnterSpecification = ();

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError> {
        Ok(self.future_runtime.run())
    }
}

use zcene_core::actor::ActorCommonBounds;

impl<H> ActorSpawnHandler<ActorHandler<H>> for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type SpawnSpecification<A>
        = A
    where
        A: ActorCommonBounds + Actor<ActorHandler<H>>;

    fn spawn<A>(
        &self,
        actor: Self::SpawnSpecification<A>,
    ) -> Result<Self::Address<A>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        self.future_runtime.spawn(ActorPrivilegedExecutor::new(
            Some(ActorPrivilegedExecutorCreateState::new(actor).into()),
            receiver,
            ActorCommonContextBuilder::default(),
            None,
        ))?;

        Ok(<Self as actor::ActorHandler>::Address::new(sender))
    }
}

use alloc::vec::Vec;
use core::marker::PhantomData;
use core::num::NonZero;

pub struct ActorUnprivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<ActorUnprivilegedHandler>,
    H: actor::ActorHandler + ActorAllocatorHandler,
{
    actor: A,
    address_mappings: Vec<()>,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    marker: PhantomData<H>,
}

impl<H> ActorSpawnHandler<ActorUnprivilegedHandler> for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type SpawnSpecification<A>
        = ActorUnprivilegedHandlerSpawnSpecification<A, Self>
    where
        A: ActorCommonBounds + Actor<ActorUnprivilegedHandler>;

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<Self::Address<A>, ActorSpawnError>
    where
        A: Actor<Self> + Actor<ActorUnprivilegedHandler>,
        <A as Actor<Self>>::Message: From<<A as Actor<ActorUnprivilegedHandler>>::Message>,
        <A as Actor<ActorUnprivilegedHandler>>::Message: From<<A as Actor<Self>>::Message>,
    {
        let (sender, receiver) =
            ActorMessageChannel::<<A as Actor<Self>>::Message>::new_unbounded();

        self.future_runtime
            .spawn(ActorUnprivilegedExecutor::<A, Self>::new(
                Some(
                    ActorUnprivilegedExecutorCreateState::new(Box::new(specification.actor), None)
                        .into(),
                ),
                receiver,
                specification.deadline_in_milliseconds,
            ))?;

        Ok(Self::Address::new(sender))
    }
}

use crate::actor::ActorUnprivilegedHandler;

impl<H> ActorDiscoverHandler for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    fn discover<M>(&self) -> Option<ActorMailbox<M, Self>>
    where
        M: ActorMessage,
    {
        None
    }
}
