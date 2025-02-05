use crate::kernel::Kernel;
use core::fmt::Write;
use x86_64::structures::idt::InterruptStackFrame;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub extern "x86-interrupt" fn double_fault_entry_point(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    let _ = Kernel::get()
        .logger()
        .writer(|w| write!(w, "Double Fault {:?}", error_code));

    loop {}
}
