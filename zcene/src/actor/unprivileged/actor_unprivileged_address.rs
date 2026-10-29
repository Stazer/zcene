use crate::actor::unprivileged::{ActorUnprivilegedEnvironment, ActorUnprivilegedRecipient};
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorAddress, ActorFuture, ActorMessageSender, ActorSendError};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Address of the root-side recipient at `index` of the message handlers an unprivileged actor was
/// spawned with, seen as a destination for the messages of the actor `A`. Use
/// `ActorUnprivilegedRecipient` instead when only the message type is known.
#[derive(Constructor)]
pub struct ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    index: usize,
    #[Constructor(default)]
    marker: PhantomData<A::Message>,
}

impl<A> Clone for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    fn clone(&self) -> Self {
        Self::new(self.index)
    }
}

impl<A> ActorAddress<A, ActorUnprivilegedEnvironment> for ActorUnprivilegedAddress<A> where
    A: Actor<ActorUnprivilegedEnvironment>
{
}

impl<A> ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    /// The same destination, but only known by the type of message it accepts.
    pub const fn recipient(&self) -> ActorUnprivilegedRecipient<A::Message> {
        ActorUnprivilegedRecipient::new(self.index)
    }
}

impl<A> ActorMessageSender<A::Message> for ActorUnprivilegedAddress<A>
where
    A: Actor<ActorUnprivilegedEnvironment>,
{
    fn send(&self, message: A::Message) -> impl ActorFuture<'_, Result<(), ActorSendError>> {
        let recipient = self.index;

        async move {
            crate::actor::unprivileged::send_message_copy(recipient, &message);

            Ok(())
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
#[cfg(test)]
fn recipient_keeps_index() {
    struct TestActor;

    impl Actor<ActorUnprivilegedEnvironment> for TestActor {
        type Message = u32;
    }

    let address = ActorUnprivilegedAddress::<TestActor>::new(3);

    assert_eq!(3, address.recipient().index());
}
