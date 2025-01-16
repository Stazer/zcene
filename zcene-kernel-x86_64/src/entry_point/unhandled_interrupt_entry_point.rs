use crate::kernel::{Kernel, TimerActorMessage};
use core::fmt::Write;
use x86::apic::x2apic::X2APIC;
use x86::apic::ApicControl;
use x86_64::structures::idt::InterruptStackFrame;
use zcene_core::actor::ActorMessageSender;
use zcene_core::future::FutureExt;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub extern "x86-interrupt" fn unhandled_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("Unhandled interrupt");

    loop {}
}

fn handle(stack_frame: InterruptStackFrame) -> ! {
    Kernel::get()
        .timer_actor()
        .send(TimerActorMessage::Tick)
        .complete();

    /*Kernel::get()
        .actor_system()
        .handler()
        .reschedule(stack_frame);*/

    //X2APIC::new().eoi();

    //x86_64::instructions::interrupts::enable();

    /*Kernel::get()
        .actor_system()
        .enter().unwrap();*/

    loop {}
}

pub extern "x86-interrupt" fn timer_interrupt_entry_point(stack_frame: InterruptStackFrame) {
    handle(stack_frame);
}

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
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
}
