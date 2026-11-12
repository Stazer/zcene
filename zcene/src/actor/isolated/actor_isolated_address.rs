use crate::actor::isolated::{
    ActorIsolatedEnvironment, ActorIsolatedEnvironmentPerspective,
    ActorIsolatedEnvironmentUnprivilegedPerspective, ActorIsolatedRecipient,
};
use zcene_core::actor::{Actor, ActorAddress, ActorFuture, ActorMessageSender, ActorSendError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedAddress<A, P = ActorIsolatedEnvironmentUnprivilegedPerspective>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorIsolatedEnvironmentPerspective,
{
    recipient: ActorIsolatedRecipient<A::Message, P>,
}

impl<A, P> ActorIsolatedAddress<A, P>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorIsolatedEnvironmentPerspective,
{
    pub fn new(recipient: ActorIsolatedRecipient<A::Message, P>) -> Self {
        Self { recipient }
    }

    pub fn recipient(&self) -> ActorIsolatedRecipient<A::Message, P> {
        self.recipient.clone()
    }
}

impl<A, P> Clone for ActorIsolatedAddress<A, P>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorIsolatedEnvironmentPerspective,
{
    fn clone(&self) -> Self {
        Self::new(self.recipient.clone())
    }
}

impl<A> ActorAddress<A, ActorIsolatedEnvironment>
    for ActorIsolatedAddress<A, ActorIsolatedEnvironmentUnprivilegedPerspective>
where
    A: Actor<ActorIsolatedEnvironment>,
{
}

impl<A, P> ActorMessageSender<A::Message> for ActorIsolatedAddress<A, P>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorIsolatedEnvironmentPerspective,
    ActorIsolatedRecipient<A::Message, P>: ActorMessageSender<A::Message>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.recipient.send(message).await }
    }
}
