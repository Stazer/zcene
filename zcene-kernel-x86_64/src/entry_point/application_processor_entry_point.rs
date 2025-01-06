use crate::kernel::Kernel;
use core::fmt::Write;
use x86::cpuid::CpuId;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[no_mangle]
pub extern "C" fn application_processor_entry_point() -> ! {
    let cpu_id = CpuId::new();
    let feature_info = cpu_id.get_feature_info().unwrap();

    Kernel::get().actor_system().enter();

    loop {}

    /*loop {
        Kernel::get().logger().writer(|w| {
            write!(
                w,
                "Hello World from {}!!!!!!!\n",
                feature_info.initial_local_apic_id()
            )
        });

        for i in 0..100000000 {
            core::hint::black_box(());
            x86_64::instructions::nop();
        }
    }*/
}
