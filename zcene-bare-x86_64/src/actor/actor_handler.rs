use zcene_core::actor::{
    Actor,
};
use zcene_core::actor::ActorEnvironment;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentTransformer<E>: Sized
where
    E: ActorEnvironment,
{
    type Output: Actor<E>;

    fn transform(self) -> Self::Output;
}

/*#[derive(Constructor)]
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

use zcene_core::actor::ActorMessageSender;*/
