use crate::kernel::Kernel;
use core::fmt::Write;
use core::panic::PanicInfo;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    let _ = Kernel::get()
        .logger()
        .writer(|w| write!(w, "Error: {:?}", panic_info));

    loop {}
}
