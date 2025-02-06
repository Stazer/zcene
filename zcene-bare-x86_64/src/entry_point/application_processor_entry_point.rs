use crate::kernel::Kernel;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[no_mangle]
pub extern "C" fn application_processor_entry_point() -> ! {
    Kernel::get().actor_system().enter().unwrap();

    loop {}
}
