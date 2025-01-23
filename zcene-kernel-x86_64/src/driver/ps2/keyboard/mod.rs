use zcene_core::actor::{ActorContextMessageProvider, Actor, ActorHandler, ActorHandleError, ActorMailbox, ActorFuture};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use alloc::vec::Vec;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct KeyboardDecodedKeyMessage(DecodedKey);

#[derive(Debug)]
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


/*use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::Port;

static KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(Keyboard::new(
    ScancodeSet1::new(),
    layouts::Us104Key,
    HandleControl::Ignore,
));

pub extern "x86-interrupt" fn keyboard_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    Kernel::get()
                        .logger()
                        .writer(|w| write!(w, "{}", character,));
                }
                DecodedKey::RawKey(key) => {
                    Kernel::get().logger().writer(|w| write!(w, "{:?}", key,));
                }
            }
        }
    }

    X2APIC::new().eoi();
}*/
