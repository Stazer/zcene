use crate::actor::{
    ActorPrivilegedExecutor, ActorPrivilegedExecutorCreateState,
    ActorUnprivilegedExecutor, ActorUnprivilegedExecutorCreateState,
};
use alloc::boxed::Box;
use zcene_core::actor::{
    self, Actor, ActorAddressReference, ActorCommonContextBuilder,
    ActorCommonHandleContext,
    ActorMailbox, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress, ActorSpawnError, ActorEnvironmentAllocator,

};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::{Constructor, Method};
use zcene_core::actor::{ActorEnvironmentSpawnable};
use zcene_core::actor::ActorEnvironmentEnterable;
use zcene_core::actor::ActorEnterError;
use zcene_core::actor::ActorCommonBounds;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::num::NonZero;

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

impl<H> ActorEnvironmentEnterable<ActorHandler<H>> for ()
where
    H: FutureRuntimeHandler,
{
    fn enter(self, environment: &ActorHandler<H>) -> Result<(), ActorEnterError> {
        environment.future_runtime.run();

        Ok(())
    }
}

/*impl<H> ActorSpawnHandler<ActorHandler<H>> for ActorHandler<H>
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
    ) -> Result<<ActorHandler<H> as actor::ActorEnvironment>::Address<A>, ActorSpawnError>
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

        Ok(<Self as actor::ActorEnvironment>::Address::new(sender))
    }
}*/

use zcene_core::actor::ActorMessageSender;

/*impl<A, H> ActorSpawnable<ActorHandler<H>> for A
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    type Actor = A;

    fn spawn(self, handler: &ActorHandler<H>) -> <ActorHandler<H> as actor::ActorEnvironment>::Address<A> {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        handler.future_runtime.spawn(ActorPrivilegedExecutor::new(
            Some(ActorPrivilegedExecutorCreateState::new(self).into()),
            receiver,
            ActorCommonContextBuilder::default(),
            None,
        )).unwrap();

        <ActorHandler<H> as actor::ActorEnvironment>::Address::new(sender)//).unwrap()
    }
}

pub struct ActorPrivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: actor::ActorEnvironment
{
    actor: A,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    marker: PhantomData<H>,
}

pub struct ActorUnprivilegedHandlerSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: actor::ActorEnvironment
{
    pub actor: A,
    pub addresses: Vec<()>,
    pub deadline_in_milliseconds: Option<NonZero<usize>>,
    pub marker: PhantomData<H>,
}

pub trait ActorEnvironmentTransformer<FE, TE>
where
    FE: actor::ActorEnvironment,
    TE: actor::ActorEnvironment,
    Self: Actor<FE>,
{
    type Output: Actor<TE>;

    fn transform(self) -> Self::Output;
}

/*impl<A, H> ActorEnvironmentTransformer<H, H> for A
where
    A: Actor<H>,
    H: actor::ActorEnvironment,
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

    fn spawn(self, handler: &ActorHandler<H>) -> <ActorHandler<H> as actor::ActorEnvironment>::Address<A> {
        self.actor.transform();

        /*let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        handler.future_runtime.spawn(ActorPrivilegedExecutor::new(
            Some(ActorPrivilegedExecutorCreateState::new(self).into()),
            receiver,
            ActorCommonContextBuilder::default(),
            None,
        )).unwrap();

        <ActorHandler<H> as actor::ActorEnvironment>::Address::new(sender)//).unwrap()*/
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
*/
