use crate::kernel::Kernel;
use core::fmt::Write;
use x86::cpuid::CpuId;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[no_mangle]
pub extern "C" fn application_processor_entry_point() -> ! {
    Kernel::get().actor_system().enter();

    loop {}
}
