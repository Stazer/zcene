use crate::platform::unprivileged_event::UnprivilegedEvent;
use core::time::Duration;
use zcene_core::future::runtime::FutureRuntimeAllocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Platform: Sized + Send + Sync + 'static {
    type BootInfo;
    type Allocator: FutureRuntimeAllocator;
    type UnprivilegedSystemCallContext: Send + Clone + 'static;
    type UnprivilegedPreemptionContext: Send + Clone + 'static;

    fn initialize(boot_info: Self::BootInfo) -> Self;

    fn allocator(&self) -> Self::Allocator;

    fn allocate_unprivileged_stack(&self) -> Option<u64>;
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
    );

    fn resume_after_system_call(
        context: &Self::UnprivilegedSystemCallContext,
        event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
    );

    fn resume_after_preemption(
        context: &Self::UnprivilegedPreemptionContext,
        event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
    );

    fn complete_unprivileged_execution() -> !;
}
