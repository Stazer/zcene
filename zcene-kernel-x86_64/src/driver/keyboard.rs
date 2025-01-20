use zcene_core::actor::{ActorContextMessageProvider, Actor, ActorHandler, ActorHandleError, ActorMailbox, ActorFuture};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use alloc::vec::Vec;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct KeyboardDecodedKeyMessage(DecodedKey);

pub enum KeyboardMessage<H>
where
    H: ActorHandler,
{
    Subscription(ActorMailbox<KeyboardDecodedKeyMessage, H>),
    DecodedKey(KeyboardDecodedKeyMessage),
}

impl<H> Clone for KeyboardMessage<H>
where
    H: ActorHandler,
{
    fn clone(&self) -> Self {
        match self {
            Self::Subscription(mailbox) => Self::Subscription(mailbox.clone()),
            Self::DecodedKey(message) => Self::DecodedKey(message.clone()),
        }
    }
}

pub struct KeyboardActor<H>
where
    H: ActorHandler,
{
    subscriptions: Vec<ActorMailbox<KeyboardDecodedKeyMessage, H>, H::Allocator>
}

impl<H> Actor<H> for KeyboardActor<H>
where
    H: ActorHandler,
    H::HandleContext<KeyboardMessage<H>>: ActorContextMessageProvider<KeyboardMessage<H>>,
{
    type Message = KeyboardMessage<H>;

    fn handle(&mut self, context: H::HandleContext<Self::Message>) -> impl ActorFuture<'_, Result<(), ActorHandleError>>  {
        async move {
            match context.message() {
                KeyboardMessage::Subscription(mailbox) => self.subscriptions.push(mailbox.clone()),
                KeyboardMessage::DecodedKey(key) => {
                }
            }
            Ok(())
        }
    }
}
