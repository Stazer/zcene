use crate::actor::ActorRootEnvironment;
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{Actor, ActorContextBuilder, ActorFuture, ActorMessageChannelReceiver};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootEnvironmentExecutor<A, B, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    B: ActorContextBuilder<A, ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    actor: A,
    receiver: ActorMessageChannelReceiver<A::Message>,
    context_builder: B,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, B, H> ActorRootEnvironmentExecutor<A, B, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    B: ActorContextBuilder<A, ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    pub async fn run(mut self) {
        if !matches!(self.deadline_in_milliseconds, None) {
            todo!()
        }

        // TODO: Handle result
        let _result = self.actor.create(()).await;

        loop {
            let message = match self.receiver.receive().await {
                Some(message) => message,
                None => break,
            };

            // TODO: Handle result
            let _result = self
                .actor
                .handle(
                    self.context_builder
                        .build_handle_context(&self.actor, &message),
                )
                .await;
        }

        // TODO: Handle result
        let _result = self.actor.destroy(()).await;
    }
}
