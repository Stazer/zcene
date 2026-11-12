use crate::platform::unprivileged_address_space::{
    UnprivilegedAddressSpace, UnprivilegedAddressSpaceSpecification,
};
use crate::platform::unprivileged_event::UnprivilegedEvent;
use core::time::Duration;
use zcene_core::future::runtime::FutureRuntimeAllocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Platform: Sized + Send + Sync + 'static {
    type BootInfo;
    type Allocator: FutureRuntimeAllocator;
    type UnprivilegedSystemCallContext: Send + Clone + 'static;
    type UnprivilegedPreemptionContext: Send + Clone + 'static;
    type UnprivilegedAddressSpace: UnprivilegedAddressSpace;

    fn initialize(boot_info: Self::BootInfo) -> Self;

    fn allocator(&self) -> Self::Allocator;

    /// Allocates at least `size` bytes of zeroed memory an unprivileged actor may read and write.
    fn allocate_unprivileged_memory(&self, size: usize) -> Option<&'static mut [u8]>;
    /// Creates an address space of its own for an unprivileged actor: `size` bytes of zeroed memory
    /// at `user_base`, the first `executable_length` of them readable and executable but not
    /// writable, the rest readable and writable. Nothing else of the system is accessible from it.
    fn create_unprivileged_address_space(
        &self,
        specification: &UnprivilegedAddressSpaceSpecification,
    ) -> Option<Self::UnprivilegedAddressSpace>;

    fn set_preemption_deadline(&self, duration: Duration);
    /// Stops a running preemption deadline, so it cannot expire while the kernel is busy with
    /// something else and then hit the next unprivileged execution immediately.
    fn clear_preemption_deadline(&self);
    fn notify_end_of_interrupt(&self);

    fn execute_in_unprivileged(
        argument: *mut (),
        message: *const (),
        event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
        stack: u64,
        entry: extern "C" fn(*mut (), *const ()) -> !,
        address_space: Option<&Self::UnprivilegedAddressSpace>,
    );

    fn resume_after_system_call(
        context: &Self::UnprivilegedSystemCallContext,
        event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
        address_space: Option<&Self::UnprivilegedAddressSpace>,
    );

    fn resume_after_preemption(
        context: &Self::UnprivilegedPreemptionContext,
        event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
        address_space: Option<&Self::UnprivilegedAddressSpace>,
    );

    fn complete_unprivileged_execution() -> !;
}
