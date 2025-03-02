use crate::actor::{
    ActorRootEnvironment, ActorRootEnvironmentExecutorCreateState,
    ActorRootEnvironmentExecutorDestroyState, ActorRootEnvironmentExecutorHandleState,
    ActorRootEnvironmentExecutorHandleStateInner, ActorRootEnvironmentExecutorReceiveState,
    ActorRootEnvironmentExecutorState,
};
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};
use pin_project::pin_project;
use zcene_core::actor::{
    Actor, ActorContextBuilder, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use zcene_core::future::FutureExt;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[pin_project]
#[derive(Constructor)]
pub struct ActorRootEnvironmentExecutor<A, B, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    B: ActorContextBuilder<A, ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    state: Option<ActorRootEnvironmentExecutorState<A, H>>,
    receiver: ActorMessageChannelReceiver<A::Message>,
    context_builder: B,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, B, H> Future for ActorRootEnvironmentExecutor<A, B, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    B: ActorContextBuilder<A, ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(_) = self.deadline_in_milliseconds {
            todo!()
        }

        loop {
            match self.state.take() {
                Some(ActorRootEnvironmentExecutorState::Create(state)) => {
                    let mut actor = state.into_inner().actor;

                    let result = {
                        let create_context = self.context_builder.build_create_context(&actor);
                        let mut pinned = pin!(actor.create(create_context));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorRootEnvironmentExecutorCreateState::new(actor).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            self.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor).into());
                        }
                    }
                }
                Some(ActorRootEnvironmentExecutorState::Receive(state)) => {
                    let actor = state.into_inner().actor;

                    let result = {
                        let mut pinned = pin!(self.receiver.receive());

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        Poll::Ready(None) => {
                            self.state =
                                Some(ActorRootEnvironmentExecutorDestroyState::new(actor).into());
                        }
                        Poll::Ready(Some(message)) => {
                            self.state = Some(
                                ActorRootEnvironmentExecutorHandleState::new(actor, message).into(),
                            );
                        }
                    }
                }
                Some(ActorRootEnvironmentExecutorState::Handle(state)) => {
                    let ActorRootEnvironmentExecutorHandleStateInner {
                        mut actor, message, ..
                    } = state.into_inner();

                    let result = {
                        let handle_context =
                            self.context_builder.build_handle_context(&actor, &message);
                        let mut pinned = pin!(actor.handle(handle_context));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            self.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor).into());
                        }
                    }
                }
                Some(ActorRootEnvironmentExecutorState::Destroy(state)) => {
                    let actor = state.into_inner().actor;
                    let destroy_context = self.context_builder.build_destroy_context(&actor);
                    let mut pinned = pin!(actor.destroy(destroy_context));

                    pinned.as_mut().complete();
                }
                None => return Poll::Ready(()),
            }
        }
    }
}
