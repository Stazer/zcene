use crate::actor::ActorRootEnvironment;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::pin;
use zcene_core::actor::{ActorFuture, Actor, ActorContextBuilder, ActorMessageChannelReceiver};
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
    //state: Option<ActorRootEnvironmentExecutorState<A, H>>,
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
            //let _result = self.actor.handle(self.context_builder.build_handle_context(&self.actor, &message)).await;

            core::future::poll_fn(|context| {
                let mut pinned = pin!(self.actor.handle(self.context_builder.build_handle_context(&self.actor, &message)));
                pinned.as_mut().poll(context)
            }).await;
        }

        // TODO: Handle result
        let _result = self.actor.destroy(()).await;
    }
}

/*impl<A, B, H> Future for ActorRootEnvironmentExecutor<A, B, H>
where
    A: Actor<ActorRootEnvironment<H>>,
    B: ActorContextBuilder<A, ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        async {
        }
        /*let a = self.receiver.receive();

        a.poll(context).map(|x| ());*/
        //use futures::FutureExt;
        //let this = self.project();

        //let mut this = self.project();
        // Falls wir bereits ein gespeichertes Future haben, pollen wir es zuerst
        if let Some(fut) = self.as_mut().pending_future.as_mut() {
            match fut.as_mut().poll(context) {
                Poll::Ready(value) => {
                    // Future ist fertig, also speichern wir kein weiteres Future mehr
                    self.as_mut().pending_future = None;
                    return Poll::Ready(());
                }
                Poll::Pending => {
                    // Future ist noch nicht fertig, wir müssen später erneut pollen
                    return Poll::Pending;
                }
            }
        }

        // Andernfalls starten wir ein neues Future
        let recv_future = self.receiver.receive();
        let mut pinned_fut = Box::pin(recv_future);

        // Direkt pollen
        match pinned_fut.as_mut().poll(context) {
            Poll::Ready(value) => Poll::Ready(value),
            Poll::Pending => {
                // Falls das Future noch nicht fertig ist, speichern wir es für das nächste Mal
                self.as_mut().pending_future = Some(pinned_fut);
                Poll::Pending
            }
        };

        if !matches!(self.deadline_in_milliseconds, None) {
            todo!()
        }

        loop {
            match this.state.take() {
                Some(ActorRootEnvironmentExecutorState::Create(state)) => {
                    let mut actor = state.into_inner().actor;

                    let result = {
                        let create_context = this.context_builder.build_create_context(&actor);
                        let mut pinned = pin!(actor.create(create_context));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *this.state =
                                Some(ActorRootEnvironmentExecutorCreateState::new(actor).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            *this.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor, None).into());
                        }
                    }
                }
                Some(ActorRootEnvironmentExecutorState::Receive(state)) => {
                    crate::kernel::logger::println!("receive...");

                    let actor = state.into_inner().actor;

                    self.receive_message();

                    //let a: Pin<Box<dyn Future<Output = Option<A::Message>> + Send + 'static>> = Box::pin(this.receiver.receive());

                    //*this.option = (Some());
                    //if this.option.is_none() {
                        //let future = this.receiver.receive(); // Kein `Box::pin()`!
                        //this.option.set(Some(Box::pin(future))); // Future direkt speichern
                    //}*/


                    //let future = this.option.get_or_insert_with(|| Box::pin(this.receiver.receive()));

                    //let mut pinned = Box::pin(this.receiver.receive());
                    //let result = pinned.as_mut().poll(context);

                    /*match result {
                        Poll::Pending => {
                            *this.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(
                                    actor,
                                    None,
                                    //Some(pinned).
                                        ).into(),
                                     );

                            return Poll::Pending;
                        }
                        Poll::Ready(None) => {
                            *this.state =
                                Some(ActorRootEnvironmentExecutorDestroyState::new(actor).into());
                        }
                        Poll::Ready(Some(message)) => {
                            crate::kernel::logger::println!("received message");

                            *this.state = Some(
                                ActorRootEnvironmentExecutorHandleState::new(actor, message).into(),
                            );
                        }
                    }*/
                }
                Some(ActorRootEnvironmentExecutorState::Handle(state)) => {
                    let ActorRootEnvironmentExecutorHandleStateInner {
                        mut actor, message, ..
                    } = state.into_inner();

                    let result = {
                        let handle_context =
                            this.context_builder.build_handle_context(&actor, &message);
                        let mut pinned = pin!(actor.handle(handle_context));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *this.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor, None).into());

                            return Poll::Pending;
                        }
                        // TODO: Handle result
                        Poll::Ready(_result) => {
                            *this.state =
                                Some(ActorRootEnvironmentExecutorReceiveState::new(actor, None).into());
                        }
                    }
                }
                Some(ActorRootEnvironmentExecutorState::Destroy(state)) => {
                    let actor = state.into_inner().actor;
                    let destroy_context = this.context_builder.build_destroy_context(&actor);
                    let mut pinned = pin!(actor.destroy(destroy_context));

                    // TODO
                    pinned.as_mut().complete();
                }
                None => break Poll::Ready(()),
            }
        }
    }
}*/
