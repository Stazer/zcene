use crate::actor::isolated::ActorIsolatedEnvironment;
use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironmentPerspective, ActorUnprivilegedEnvironmentUnprivilegedPerspective,
    ActorUnprivilegedRecipient,
};
use zcene_core::actor::{Actor, ActorAddress, ActorFuture, ActorMessageSender, ActorSendError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedAddress<A, P = ActorUnprivilegedEnvironmentUnprivilegedPerspective>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    recipient: ActorUnprivilegedRecipient<A::Message, P>,
}

impl<A, P> ActorIsolatedAddress<A, P>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    pub fn new(recipient: ActorUnprivilegedRecipient<A::Message, P>) -> Self {
        Self { recipient }
    }

    pub fn recipient(&self) -> ActorUnprivilegedRecipient<A::Message, P> {
        self.recipient.clone()
    }
}

impl<A, P> Clone for ActorIsolatedAddress<A, P>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    fn clone(&self) -> Self {
        Self::new(self.recipient.clone())
    }
}

impl<A> ActorAddress<A, ActorIsolatedEnvironment>
    for ActorIsolatedAddress<A, ActorUnprivilegedEnvironmentUnprivilegedPerspective>
where
    A: Actor<ActorIsolatedEnvironment>,
{
}

impl<A, P> ActorMessageSender<A::Message> for ActorIsolatedAddress<A, P>
where
    A: Actor<ActorIsolatedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
    ActorUnprivilegedRecipient<A::Message, P>: ActorMessageSender<A::Message>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.recipient.send(message).await }
    }
}
