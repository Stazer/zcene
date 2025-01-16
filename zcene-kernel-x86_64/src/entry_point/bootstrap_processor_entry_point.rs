use crate::kernel::Kernel;
use bootloader_api::BootInfo;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use core::fmt::Write;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn bootstrap_processor_entry_point(boot_info: &'static mut BootInfo) -> ! {
    let kernel = Kernel::get_mut();

    if let Err(error) = kernel.initialize(boot_info) {
        let _ = kernel.logger().writer(|w| write!(w, "Error: {:?}", error));
        loop {}
    }

    kernel.run()
}
