use crate::common::memory::allocator::{ErasedAllocator, SelfContainedHeapAllocator};
use crate::common::time::AtomicTimer;
use crate::platform::hosted::HostedLoggerService;
use crate::platform::unprivileged_event::UnprivilegedEvent;
use crate::platform::Platform;
use alloc::alloc::Global;
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct HostedPlatform {
    logger: HostedLoggerService,
    timer: AtomicTimer,
    allocator: ErasedAllocator,
}

impl HostedPlatform {
    pub fn logger(&self) -> &HostedLoggerService {
        &self.logger
    }

    pub fn timer(&self) -> &AtomicTimer {
        &self.timer
    }
}

impl Platform for HostedPlatform {
    type BootInfo = ();
    type Allocator = ErasedAllocator;
    type UnprivilegedSystemCallContext = ();
    type UnprivilegedPreemptionContext = ();

    fn allocator(&self) -> Self::Allocator {
        self.allocator.clone()
    }

    fn initialize(_boot_info: Self::BootInfo) -> Self {
        let heap_allocator = SelfContainedHeapAllocator::new(Global)
            .unwrap_or_else(|error| panic!("failed to initialize hosted allocator: {:?}", error));

        Self {
            logger: HostedLoggerService::new(),
            timer: AtomicTimer::default(),
            allocator: ErasedAllocator::new(heap_allocator),
        }
    }

    fn allocate_unprivileged_stack(&self) -> Option<u64> {
        todo!("hosted platform does not support unprivileged execution")
    }

    fn set_preemption_deadline(&self, _duration: Duration) {
        todo!("hosted platform does not support preemption")
    }

    fn clear_preemption_deadline(&self) {
        todo!("hosted platform does not support preemption")
    }

    fn notify_end_of_interrupt(&self) {
        todo!("hosted platform does not support interrupts")
    }

    fn execute_in_unprivileged(
        _argument: *mut (),
        _message: *const (),
        _event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
        _stack: u64,
        _entry: extern "C" fn(*mut (), *const ()) -> !,
    ) {
        todo!("hosted platform does not support unprivileged execution")
    }

    fn resume_after_system_call(
        _context: &Self::UnprivilegedSystemCallContext,
        _event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
    ) {
        todo!("hosted platform does not support unprivileged execution")
    }

    fn resume_after_preemption(
        _context: &Self::UnprivilegedPreemptionContext,
        _event: &mut Option<
            UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>,
        >,
    ) {
        todo!("hosted platform does not support unprivileged execution")
    }

    fn complete_unprivileged_execution() -> ! {
        todo!("hosted platform does not support unprivileged execution")
    }
}
