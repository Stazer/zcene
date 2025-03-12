use crate::actor::{ActorRootEnvironment, ActorRootEnvironmentCreateContext, ActorRootEnvironmentCreateContext2};
use core::marker::PhantomData;
use core::num::NonZero;
use zcene_core::actor::{Actor, ActorMessageChannelReceiver, ActorSystemReference};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorRootEnvironmentExecutor<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    system: ActorSystemReference<ActorRootEnvironment<H>>,
    actor: A,
    receiver: ActorMessageChannelReceiver<A::Message>,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, H> ActorRootEnvironmentExecutor<A, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    pub async fn run(mut self) {
        if !matches!(self.deadline_in_milliseconds, None) {
            todo!()
        }

        // TODO: Handle result
        let _result = self
            .actor
            .create(ActorRootEnvironmentCreateContext::new(self.system.clone()))
            .await;

        let _result = self
            .actor
            .create2(ActorRootEnvironmentCreateContext2::new(&self.system))
            .await;

        /*let _result = self
            .actor
            .create3(ActorRootEnvironmentCreateContext2::new(&self.system))
            .await;*/

        loop {
            let message = match self.receiver.receive().await {
                Some(message) => message,
                None => break,
            };

            // TODO: Handle result
            /*let _result = self
            .actor
            .handle(())
            .await;*/
        }

        // TODO: Handle result
        let _result = self.actor.destroy(()).await;
    }
}
