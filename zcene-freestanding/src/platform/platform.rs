use crate::platform::unprivileged_event::UnprivilegedEvent;
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Platform: Sized + Send + Sync + 'static {
    type BootInfo;
    type UnprivilegedSystemCallContext: Send + Clone + 'static;
    type UnprivilegedPreemptionContext: Send + Clone + 'static;

    fn initialize(boot_info: Self::BootInfo) -> Self;

    fn allocate_unprivileged_stack(&self) -> Option<u64>;
    fn set_preemption_deadline(&self, duration: Duration);
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
