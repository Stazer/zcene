use crate::actor::{
    ActorPrivilegedExecutorCreateState, ActorPrivilegedExecutorDestroyState,
    ActorPrivilegedExecutorHandleState, ActorPrivilegedExecutorHandleStateInner,
    ActorPrivilegedExecutorReceiveState, ActorPrivilegedExecutorState,
};
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};
use pin_project::pin_project;
use zcene_core::actor::{Actor, ActorContextBuilder, ActorEnvironment, ActorMessageChannelReceiver};
use zcene_core::future::FutureExt;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[pin_project]
#[derive(Constructor)]
pub struct ActorPrivilegedExecutor<A, B, E>
where
    A: Actor<E>,
    B: ActorContextBuilder<A, E>,
    E: ActorEnvironment,
{
    state: Option<ActorPrivilegedExecutorState<A, E>>,
    receiver: ActorMessageChannelReceiver<A::Message>,
    context_builder: B,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<E>,
}

impl<A, B, E> Future for ActorPrivilegedExecutor<A, B, E>
where
    A: Actor<E>,
    B: ActorContextBuilder<A, E>,
    E: ActorEnvironment,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(_) = self.deadline_in_milliseconds {
            todo!()
        }

        loop {
            match self.state.take() {
                Some(ActorPrivilegedExecutorState::Create(state)) => {
                    let mut actor = state.into_inner().actor;

                    let result = {
                        let create_context = self.context_builder.build_create_context(&actor);
                        let mut pinned = pin!(actor.create(create_context));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorPrivilegedExecutorCreateState::new(actor).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            self.state =
                                Some(ActorPrivilegedExecutorReceiveState::new(actor).into());
                        }
                    }
                }
                Some(ActorPrivilegedExecutorState::Receive(state)) => {
                    let actor = state.into_inner().actor;

                    let result = {
                        let mut pinned = pin!(self.receiver.receive());

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorPrivilegedExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        Poll::Ready(None) => {
                            self.state =
                                Some(ActorPrivilegedExecutorDestroyState::new(actor).into());
                        }
                        Poll::Ready(Some(message)) => {
                            self.state = Some(
                                ActorPrivilegedExecutorHandleState::new(actor, message).into(),
                            );
                        }
                    }
                }
                Some(ActorPrivilegedExecutorState::Handle(state)) => {
                    let ActorPrivilegedExecutorHandleStateInner {
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
                                Some(ActorPrivilegedExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            self.state =
                                Some(ActorPrivilegedExecutorReceiveState::new(actor).into());
                        }
                    }
                }
                Some(ActorPrivilegedExecutorState::Destroy(state)) => {
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
