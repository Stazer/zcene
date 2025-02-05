use crate::kernel::{Kernel, KERNEL};
use bootloader_api::BootInfo;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use core::fmt::Write;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn bootstrap_processor_entry_point(boot_info: &'static mut BootInfo) -> ! {
    unsafe {
        KERNEL
            .get()
            .as_mut()
            .unwrap()
            .write(Kernel::new(boot_info).unwrap());
    }

    Kernel::get().run();
}
