use crate::actor::ActorRootEnvironment;
use alloc::vec::Vec;
use pc_keyboard::DecodedKey;
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorEnvironmentAllocator, ActorFuture, ActorHandleError,
    ActorMailbox,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct KeyboardDecodedKeyMessage(DecodedKey);

#[derive(Clone, Debug)]
pub enum KeyboardMessage {
    Subscription(ActorMailbox<KeyboardDecodedKeyMessage, ActorRootEnvironment>),
    Byte(u16),
}

pub struct KeyboardActor {
    subscriptions: Vec<
        ActorMailbox<KeyboardDecodedKeyMessage, ActorRootEnvironment>,
        <ActorRootEnvironment as ActorEnvironmentAllocator>::Allocator,
    >,
}

impl KeyboardActor {
    pub fn new(allocator: <ActorRootEnvironment as ActorEnvironmentAllocator>::Allocator) -> Self {
        Self {
            subscriptions: Vec::new_in(allocator),
        }
    }
}

impl Actor<ActorRootEnvironment> for KeyboardActor {
    type Message = KeyboardMessage;

    fn handle<'a>(
        &'a mut self,
        context: <ActorRootEnvironment as ActorEnvironment>::HandleContext<'a, Self::Message>,
    ) -> impl ActorFuture<'a, Result<(), ActorHandleError>> {
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
