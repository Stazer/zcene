use crate::actor::unprivileged::ActorUnprivilegedEnvironment;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorAddress, ActorFuture, ActorMessageSender, ActorSendError};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    recipient: usize,
    #[Constructor(default)]
    marker: PhantomData<A::Message>,
}

impl<A> Clone for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    fn clone(&self) -> Self {
        Self::new(self.recipient)
    }
}

impl<A> ActorAddress<A, ActorUnprivilegedEnvironment> for ActorUnprivilegedAddress<A> where
    A: Actor<ActorUnprivilegedEnvironment>
{
}

impl<A> ActorMessageSender<A::Message> for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        let recipient = self.recipient;

        async move {
            crate::actor::unprivileged::send_message_copy(recipient, &message);

            Ok(())
        }
    }
}
