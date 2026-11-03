use crate::actor::unprivileged::{
    ActorUnprivilegedEnvironment, ActorUnprivilegedEnvironmentPerspective, ActorUnprivilegedRecipient,
    ActorUnprivilegedEnvironmentUnprivilegedPerspective,
};
use zcene_core::actor::{
    Actor, ActorAddress, ActorFuture, ActorMessageSender, ActorSendError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedAddress<A, P = ActorUnprivilegedEnvironmentUnprivilegedPerspective>
where
    A: Actor<ActorUnprivilegedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    recipient: ActorUnprivilegedRecipient<A::Message, P>,
}

impl<A, P> ActorUnprivilegedAddress<A, P>
where
    A: Actor<ActorUnprivilegedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    pub fn new(recipient: ActorUnprivilegedRecipient<A::Message, P>) -> Self {
        Self { recipient }
    }

    pub fn recipient(&self) -> ActorUnprivilegedRecipient<A::Message, P> {
        self.recipient.clone()
    }
}

impl<A, P> Clone for ActorUnprivilegedAddress<A, P>
where
    A: Actor<ActorUnprivilegedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
{
    fn clone(&self) -> Self {
        Self::new(self.recipient.clone())
    }
}

impl<A> ActorAddress<A, ActorUnprivilegedEnvironment>
    for ActorUnprivilegedAddress<A, ActorUnprivilegedEnvironmentUnprivilegedPerspective>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
}

impl<A, P> ActorMessageSender<A::Message> for ActorUnprivilegedAddress<A, P>
where
    A: Actor<ActorUnprivilegedEnvironment>,
    P: ActorUnprivilegedEnvironmentPerspective,
    ActorUnprivilegedRecipient<A::Message, P>: ActorMessageSender<A::Message>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        async move { self.recipient.send(message).await }
    }
}
