use alloc::vec::Vec;
use pc_keyboard::DecodedKey;
use zcene_core::actor::{
    Actor, ActorContextMessageProvider, ActorEnvironment, ActorEnvironmentAllocator, ActorFuture,
    ActorHandleError, ActorMailbox,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct KeyboardDecodedKeyMessage(DecodedKey);

#[derive(Debug)]
pub enum KeyboardMessage<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    Subscription(ActorMailbox<KeyboardDecodedKeyMessage, E>),
    Byte(u16),
}

impl<E> Clone for KeyboardMessage<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    fn clone(&self) -> Self {
        match self {
            Self::Subscription(mailbox) => Self::Subscription(mailbox.clone()),
            Self::Byte(byte) => Self::Byte(*byte),
        }
    }
}

pub struct KeyboardActor<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    subscriptions: Vec<ActorMailbox<KeyboardDecodedKeyMessage, E>, E::Allocator>,
}

impl<E> Actor<E> for KeyboardActor<E>
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
    E::HandleContext<KeyboardMessage<E>>: ActorContextMessageProvider<KeyboardMessage<E>>,
{
    type Message = KeyboardMessage<E>;

    fn handle(
        &mut self,
        context: E::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
            match context.message() {
                KeyboardMessage::Subscription(mailbox) => self.subscriptions.push(mailbox.clone()),
                KeyboardMessage::Byte(_byte) => {}
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
