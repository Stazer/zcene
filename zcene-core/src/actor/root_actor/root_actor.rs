use crate::actor::{
    Actor, ActorAllocatorHandler, ActorCreateError, ActorFuture, ActorHandleError, ActorEnvironment,
    ActorMailbox, RootActorMessage,
};
use alloc::vec::Vec;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct RootActor<E>
where
    E: ActorEnvironment + ActorAllocatorHandler,
{
    children: Vec<ActorMailbox<(), E>, E::Allocator>,
}

impl<E> Actor<E> for RootActor<E>
where
    E: ActorEnvironment + ActorAllocatorHandler,
    E::CreateContext: Into<ActorMailbox<(), E>>,
    E::HandleContext<RootActorMessage<E>>: Into<RootActorMessage<E>>,
{
    type Message = RootActorMessage<E>;

    fn create(
        &mut self,
        context: E::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        async move {
            self.children.push(context.into());

            Ok(())
        }
    }

    fn handle(
        &mut self,
        context: E::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
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
