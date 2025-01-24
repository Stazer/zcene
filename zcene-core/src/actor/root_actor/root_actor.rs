use crate::actor::{Actor, ActorHandler, ActorFuture, ActorHandleError, RootActorMessage, ActorCreateError, ActorMailbox};
use alloc::vec::Vec;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct RootActor<H>
where
    H: ActorHandler
{
    children: Vec<ActorMailbox<(), H>, H::Allocator>,
}

impl<H> Actor<H> for RootActor<H>
where
    H: ActorHandler,
    H::CreateContext: Into<ActorMailbox<(), H>>,
    H::HandleContext<RootActorMessage<H>>: Into<RootActorMessage<H>>,
{
    type Message = RootActorMessage<H>;

    fn create(&mut self, context: H::CreateContext) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        async move {
            self.children.push(context.into());

            Ok(())
        }
    }

    fn handle(&mut self, context: H::HandleContext<Self::Message>) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
            match context.into() {
                Self::Message::Attach(mailbox) => {
                    self.children.push(mailbox);
                }
            }

            Ok(())
        }
    }
}
