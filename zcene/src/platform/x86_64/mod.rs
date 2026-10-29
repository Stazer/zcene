pub mod smp;

mod x86_64_interrupt_manager;
pub use x86_64_interrupt_manager::*;

mod x86_64_unprivileged_system_call_context;
pub use x86_64_unprivileged_system_call_context::*;

mod x86_64_unprivileged_preemption_context;
pub use x86_64_unprivileged_preemption_context::*;

pub mod x86_64_unprivileged;
pub use x86_64_unprivileged::{
    x86_64_deadline_preemption_entry_point, x86_64_system_call_entry_point,
};

mod x86_64_memory_manager;
pub use x86_64_memory_manager::*;

mod x86_64_execution_unit;
pub use x86_64_execution_unit::*;

mod x86_64_memory_manager_initialize_error;
pub use x86_64_memory_manager_initialize_error::*;

mod timer_service;
pub use timer_service::*;

mod x86_64_platform;
pub use x86_64_platform::*;

use x86::cpuid::CpuId;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! x86_64_panic_handler {
    () => {
        #[panic_handler]
        fn panic_handler(panic_info: &::core::panic::PanicInfo) -> ! {
            use ::core::fmt::Write;

            let _ = write!(
                $crate::actor::ActorRootEnvironment::get().logger(),
                "Panic: {:?}",
                panic_info,
            );

            loop {}
        }
    };
}

pub use x86_64_panic_handler;

#[macro_export]
macro_rules! x86_64_entry_point {
    ($root_actor:expr) => {
        static BOOTLOADER_CONFIG: $crate::r#extern::bootloader_api::BootloaderConfig = {
            let mut config = $crate::r#extern::bootloader_api::BootloaderConfig::new_default();
            config.kernel_stack_size = 4 * 4096;
            config.mappings.physical_memory =
                Some($crate::r#extern::bootloader_api::config::Mapping::Dynamic);

            config
        };

        fn __x86_64_entry_point(
            boot_info: &'static mut $crate::r#extern::bootloader_api::BootInfo,
        ) -> ! {
            use $crate::ACTOR_ROOT_ENVIRONMENT;
            use $crate::actor::ActorRootEnvironment;
            use $crate::future::runtime::FutureRuntimeHandler;

            let environment =
                ActorRootEnvironment::root(FutureRuntimeHandler::new, boot_info);

            unsafe {
                ACTOR_ROOT_ENVIRONMENT
                    .get()
                    .as_mut()
                    .unwrap()
                    .write(environment.clone());
            }

            let address = environment.spawn($root_actor).unwrap();

            environment.platform().start_application_processors();

            environment.enter().unwrap();

            loop {}
        }

        $crate::r#extern::bootloader_api::entry_point!(__x86_64_entry_point, config = &BOOTLOADER_CONFIG);
    };
}

pub use x86_64_entry_point;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn initial_local_apic_id() -> Option<usize> {
    CpuId::new()
        .get_feature_info()
        .map(|feature_info| feature_info.initial_local_apic_id().into())
}

pub type ExecutionUnitIdentifier = usize;

pub fn current_execution_unit_identifier() -> ExecutionUnitIdentifier {
    CpuId::new()
        .get_feature_info()
        .map(|feature_info| feature_info.initial_local_apic_id().into())
        .unwrap_or_default()
}

#[derive(Debug)]
pub enum ExecutionUnitMode {
    Privileged,
    Unprivileged,
}
////////////////////////////////////////////////////////////////////////////////////////////////////

mod frame_size;
pub use frame_size::*;

