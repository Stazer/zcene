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

pub extern "x86-interrupt" fn non_maskable_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("non_maskable interrupt");

    loop {}
}

pub extern "x86-interrupt" fn debug_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("debug interrupt");

    loop {}
}

pub extern "x86-interrupt" fn invalid_opcode_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("invalid opcode");

    loop {}
}
pub extern "x86-interrupt" fn device_not_available_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("device not available");

    loop {}
}


pub extern "x86-interrupt" fn overflow_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("overflow interrupt");

    loop {}
}

pub extern "x86-interrupt" fn breakpoint_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("breakpoint interrupt");

    loop {}
}


pub extern "x86-interrupt" fn bound_range_exceeded_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("bound");

    loop {}
}
pub extern "x86-interrupt" fn unhandled_interrupt_with_error_code_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "unhandled {:#?}", err));


    loop {}
}

pub extern "x86-interrupt" fn invalid_tss_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "invalid tss {:#?}", err));


    loop {}
}

pub extern "x86-interrupt" fn segment_not_present_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "segment {:#?}", err));


    loop {}
}

pub extern "x86-interrupt" fn stack_segment_fault_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "stack {:#?}", err));


    loop {}
}

pub extern "x86-interrupt" fn general_protection_fault_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "protection fault {:#?}", err));


    loop {}
}

pub extern "x86-interrupt" fn alignment_check_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "alignment check {:#?}", err));


    loop {}
}

pub extern "x86-interrupt" fn virtualization_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "virt"));

    loop {}
}

pub extern "x86-interrupt" fn cp_protection_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "cp protection {}", err));

    loop {}
}


pub extern "x86-interrupt" fn hv_injection_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "hv injection"));

    loop {}
}

pub extern "x86-interrupt" fn vmm_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "vmm {:#?}", err));

    loop {}
}

pub extern "x86-interrupt" fn security_exception_entry_point(_stack_frame: InterruptStackFrame, err: u64) {
    Kernel::get()
        .logger()
        .writer(|w| write!(w, "security {:#?}", err));

    loop {
    }
}
pub extern "x86-interrupt" fn divide_by_zero_interrupt_entry_point(_stack_frame: InterruptStackFrame) {
    Kernel::get().logger().write("divide by zero");

    loop {}
}

pub extern "x86-interrupt" fn machine_check_interrupt_entry_point(_stack_frame: InterruptStackFrame) -> ! {
    Kernel::get().logger().write("Unhandled machine check");

    loop {}
}

#[no_mangle]
pub extern "C" fn timer_interrupt_handler() {
    let sp = x86::bits64::registers::rsp();

    X2APIC::new().eoi();

    Kernel::get()
        .timer_actor()
        .send(TimerActorMessage::Tick)
        .complete();

    let context = Kernel::get()
        .actor_system()
        .handler()
        .reschedule(after_preemption as _, sp);

    match context {
        Some((sp, ip)) => {
            Kernel::get().logger().writer(|w| write!(w, "restoring {:X} {:X}...\n", sp, ip));

            /*unsafe {

                core::arch::asm!(
                    "mov rsp, {sp}",
                    "add rsp, 16",
                    "jmp [{ip}]",
                    sp = in(reg) sp,
                    ip = in(reg) ip,
                    options(noreturn)
                )

            }*/
        }
        None => {

        }
    }
}

extern "C" {
    pub fn timer_interrupt_entry_point();
    pub fn after_preemption() -> !;
    pub fn start_execution(rsp: u64, rip: u64) -> !;
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
