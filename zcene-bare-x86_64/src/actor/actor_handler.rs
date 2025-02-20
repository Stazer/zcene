use crate::actor::{
    ActorInlineExecutor, ActorInlineExecutorCreateState, ActorPrivilegedExecutor,
    ActorPrivilegedExecutorCreateState, ActorSpawnSpecification, ActorSpawnSpecificationInner,
    ActorSpawnSpecificationType, ActorUnprivilegedExecutor, ActorUnprivilegedExecutorCreateState,
};
use zcene_core::actor::{
    self, Actor, ActorAddressReference, ActorCommonContextBuilder, ActorCommonHandleContext,
    ActorEnterError, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress,
    ActorSpawnError,
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

    type Allocator = <H as FutureRuntimeHandler>::Allocator;

    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();

    type SpawnSpecification<A>
        = ActorSpawnSpecification<A, Self>
    where
        A: Actor<Self>;

    type EnterSpecification = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            <Self as actor::ActorHandler>::Address::new(sender),
            self.allocator().clone(),
        )?;

        let ActorSpawnSpecificationInner { actor, r#type, .. } = specification.into_inner();

        match r#type {
            ActorSpawnSpecificationType::Inline(_) => {
                self.future_runtime.spawn(ActorInlineExecutor::new(
                    Some(ActorInlineExecutorCreateState::new(actor).into()),
                    receiver,
                    ActorCommonContextBuilder::default(),
                ))?
            }
            ActorSpawnSpecificationType::Privileged(_) => {
                self.future_runtime.spawn(ActorPrivilegedExecutor::new(
                    Some(ActorPrivilegedExecutorCreateState::new(actor).into()),
                    receiver,
                    ActorCommonContextBuilder::default(),
                    None,
                ))?
            }
            ActorSpawnSpecificationType::Unprivileged(_) => {
                self.future_runtime.spawn(ActorUnprivilegedExecutor::new(
                    Some(ActorUnprivilegedExecutorCreateState::new(actor).into()),
                    receiver,
                    ActorCommonContextBuilder::default(),
                    None,
                ))?
            }
        };

        Ok(reference)
    }

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}
