use crate::kernel::Kernel;
use core::fmt::Write;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::PageFaultErrorCode;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub extern "x86-interrupt" fn page_fault_entry_point(
    _stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let _ = Kernel::get()
        .logger()
        .writer(|w| write!(w, "Page Fault {:?}", error_code,));

    loop {}
}
