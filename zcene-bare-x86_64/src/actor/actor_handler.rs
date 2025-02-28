use crate::actor::{
    ActorPrivilegedExecutor, ActorPrivilegedExecutorCreateState,
    ActorUnprivilegedExecutor, ActorUnprivilegedExecutorCreateState,
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
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::num::NonZero;

impl<H> ActorSpawnHandler<ActorHandler<H>> for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type SpawnSpecification<A>
        = A
    where
        A: ActorCommonBounds + Actor<Self>;

    fn spawn<A>(
        &self,
        actor: Self::SpawnSpecification<A>,
    ) -> Result<<ActorHandler<H> as actor::ActorHandler>::Address<A>, ActorSpawnError>
    where
        A: Actor<ActorHandler<H>>,
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

use zcene_core::actor::ActorMessageSender;

pub trait ActorSpawnable<H>
where
    H: actor::ActorHandler,
{
    type Actor: Actor<H>;

    fn spawn(self, handler: &H) -> H::Address<Self::Actor>;
}

impl<H> ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    pub fn spawn_custom<S>(&self, spawnable: S) -> <ActorHandler<H> as actor::ActorHandler>::Address<S::Actor>
    where
        S: ActorSpawnable<Self>,
    {
        spawnable.spawn(self)
    }
}

impl<A, H> ActorSpawnable<ActorHandler<H>> for A
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    type Actor = A;

    fn spawn(self, handler: &ActorHandler<H>) -> <ActorHandler<H> as actor::ActorHandler>::Address<A> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        handler.future_runtime.spawn(ActorPrivilegedExecutor::new(
            Some(ActorPrivilegedExecutorCreateState::new(self).into()),
            receiver,
            ActorCommonContextBuilder::default(),
            None,
        )).unwrap();

        <ActorHandler<H> as actor::ActorHandler>::Address::new(sender)//).unwrap()
    }
}

pub struct ActorPrivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: actor::ActorHandler
{
    actor: A,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    marker: PhantomData<H>,
}

pub struct ActorUnprivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: actor::ActorHandler
{
    pub actor: A,
    pub addresses: Vec<()>,
    pub deadline_in_milliseconds: Option<NonZero<usize>>,
    pub marker: PhantomData<H>,
}

pub trait ActorEnvironmentTransformer<FE, TE>
where
    FE: actor::ActorHandler,
    TE: actor::ActorHandler,
    Self: Actor<FE>,
{
    type Output: Actor<TE>;

    fn transform(self) -> Self::Output;
}

/*impl<A, H> ActorEnvironmentTransformer<H, H> for A
where
    A: Actor<H>,
    H: actor::ActorHandler,
{
    type Output = A;

    fn transform(self) -> Self::Output {
        self
    }
}*/

impl<A, H> ActorSpawnable<ActorHandler<H>> for ActorUnprivilegedHandlerSpawnSpecification<A, ActorHandler<H>>
where
    A: Actor<ActorHandler<H>> + ActorEnvironmentTransformer<ActorHandler<H>, ActorUnprivilegedHandler>,
    H: FutureRuntimeHandler,
{
    type Actor = A;

    fn spawn(self, handler: &ActorHandler<H>) -> <ActorHandler<H> as actor::ActorHandler>::Address<A> {
        self.actor.transform();

        /*let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        handler.future_runtime.spawn(ActorPrivilegedExecutor::new(
            Some(ActorPrivilegedExecutorCreateState::new(self).into()),
            receiver,
            ActorCommonContextBuilder::default(),
            None,
        )).unwrap();

        <ActorHandler<H> as actor::ActorHandler>::Address::new(sender)//).unwrap()*/
        todo!()
    }
}

/*impl<H> ActorSpawnHandler<ActorUnprivilegedHandler> for ActorHandler<H>
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
        A: Actor<T>,
        //<A as Actor<Self>>::Message: From<<A as Actor<ActorUnprivilegedHandler>>::Message>,
        //<A as Actor<ActorUnprivilegedHandler>>::Message: From<<A as Actor<Self>>::Message>,
    {
        let (sender, receiver) =
            ActorMessageChannel::<<A as Actor<T>>::Message>::new_unbounded();

        self.future_runtime
            .spawn(ActorUnprivilegedExecutor::<A, Self>::new(

                    ActorUnprivilegedExecutorCreateState::new(Box::new(specification.actor), None)
                        .into(),
                ),
                receiver,
                specification.deadline_in_milliseconds,
            ))?;

        Ok(Self::Address::new(sender))
    }
}*/

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
