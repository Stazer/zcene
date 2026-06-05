pub mod smp;

mod x86_64_interrupt_manager;
pub use x86_64_interrupt_manager::*;

mod x86_64_memory_service;
pub use x86_64_memory_service::*;

mod x86_64_memory_service_initialize_error;
pub use x86_64_memory_service_initialize_error::*;

mod timer_service;
pub use timer_service::*;

mod x86_64_platform;
pub use x86_64_platform::*;

use x86::cpuid::CpuId;

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

////////////////////////////////////////////////////////////////////////////////////////////////////

mod stack;
pub use stack::*;
