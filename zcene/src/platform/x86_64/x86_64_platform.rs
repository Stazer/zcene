use crate::common::logger::LoggerService;
use crate::platform::unprivileged_event::UnprivilegedEvent;
use crate::platform::x86_64::x86_64_unprivileged::{
    x86_64_deadline_preemption_entry_point, x86_64_execute_in_unprivileged,
    x86_64_resume_after_preemption, x86_64_resume_after_system_call,
};
use crate::platform::x86_64::{
    InterruptEntryPoint, X86_64InterruptManager, X86_64LocalInterruptManager, X86_64MemoryManager,
    X86_64UnprivilegedPreemptionContext, X86_64UnprivilegedSystemCallContext, TimerService,
};
use crate::platform::Platform;
use core::fmt::Write;
use crate::common::memory::allocator::ErasedAllocator;
use crate::common::synchronization::Mutex;
use crate::platform::x86_64::{current_execution_unit_identifier, ExecutionUnitIdentifier, X86_64ExecutionUnit};
use alloc::collections::BTreeMap;
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct X86_64Platform {
    logger: LoggerService,
    memory: X86_64MemoryManager,
    timer: TimerService,
    interrupt_manager: X86_64InterruptManager,
    rsdp_address: Option<u64>,
    // Kept alive: the GDTR/TR of the bootstrap core point into it.
    #[allow(dead_code)]
    execution_unit: X86_64ExecutionUnit,
    // Prepared by the bootstrap core before application processors start, so they do not have
    // to allocate frames concurrently. Never removed: their GDTR/TR point into them.
    application_execution_units:
        Mutex<BTreeMap<ExecutionUnitIdentifier, X86_64ExecutionUnit, ErasedAllocator>>,
}

impl X86_64Platform {
    pub fn logger(&self) -> &LoggerService {
        &self.logger
    }

    pub fn memory(&self) -> &X86_64MemoryManager {
        &self.memory
    }

    pub fn timer(&self) -> &TimerService {
        &self.timer
    }

    pub fn interrupt_manager(&self) -> &X86_64InterruptManager {
        &self.interrupt_manager
    }

    pub fn rsdp_address(&self) -> Option<u64> {
        self.rsdp_address
    }

    /// Brings up every other logical core found in the ACPI MADT (INIT-SIPI-SIPI) and sets
    /// each one on a path to `crate::platform::x86_64::smp::application_processor_entry_point`.
    ///
    /// Must only be called after `ACTOR_ROOT_ENVIRONMENT` has been written: application
    /// processors read it as soon as they land in Rust.
    pub fn start_application_processors(&self) {
        crate::platform::x86_64::smp::start_application_processors(self);
    }

    /// Prepares the per-core state for application processor `identifier`.
    pub(crate) fn prepare_application_execution_unit(&self, identifier: ExecutionUnitIdentifier) -> Option<()> {
        let execution_unit = X86_64ExecutionUnit::new(&self.memory)?;
        self.application_execution_units.lock().insert(identifier, execution_unit);
        Some(())
    }

    /// Called on an application processor right after it boots: loads its GDT/TSS/syscall MSRs
    /// and sets up its local APIC timer for preemption.
    pub(crate) fn initialize_application_processor(&self) {
        let identifier = current_execution_unit_identifier();

        if let Some(execution_unit) = self.application_execution_units.lock().get(&identifier) {
            // SAFETY: each identifier is loaded exactly once, by its own core, and the entry is
            // never removed.
            unsafe { execution_unit.load() };
        }

        self.interrupt_manager.initialize_application_processor_local_apic();
    }
}

impl Platform for X86_64Platform {
    type BootInfo = &'static mut bootloader_api::BootInfo;
    type Allocator = ErasedAllocator;
    type UnprivilegedSystemCallContext = X86_64UnprivilegedSystemCallContext;
    type UnprivilegedPreemptionContext = X86_64UnprivilegedPreemptionContext;

    fn allocator(&self) -> Self::Allocator {
        self.memory.heap_allocator()
    }

    fn initialize(boot_info: Self::BootInfo) -> Self {
        let rsdp_address = boot_info.rsdp_addr.into_option();

        let logger = LoggerService::new(
            boot_info.framebuffer.take().map(|frame_buffer| {
                let info = frame_buffer.info().clone();

                bootloader_x86_64_common::framebuffer::FrameBufferWriter::new(
                    frame_buffer.into_buffer(),
                    info,
                )
            }),
            Some(unsafe { bootloader_x86_64_common::serial::SerialPort::init() }),
        );

        let memory = match X86_64MemoryManager::new(boot_info, &logger) {
            Ok(memory) => memory,
            Err(error) => {
                let _ = write!(logger.writer(), "{:?}", error);
                todo!();
            }
        };

        let timer = boot_info
            .rsdp_addr
            .into_option()
            .and_then(|rsdp_address| TimerService::from_hpet(rsdp_address, &memory))
            .unwrap_or_else(TimerService::new);

        let execution_unit = X86_64ExecutionUnit::new(&memory).unwrap();
        unsafe { execution_unit.load() };

        // The naked handler takes no typed argument (it reads the interrupt stack frame
        // itself via rsp), so its address is reinterpreted as the x86-interrupt ABI
        // function pointer the IDT expects.
        let deadline_preemption_entry_point = unsafe {
            core::mem::transmute::<unsafe extern "C" fn(), InterruptEntryPoint>(
                x86_64_deadline_preemption_entry_point,
            )
        };

        // Requires the TSS (loaded above) to already be in place: allocated interrupt
        // vectors are pinned to IST slot 0, i.e. `timer_stack`.
        let mut local_interrupt_manager = X86_64LocalInterruptManager::new(&timer, &memory);
        // Configured but not started: unprivileged executors arm it per run.
        local_interrupt_manager
            .enable_oneshot(deadline_preemption_entry_point, Duration::ZERO);

        let mut interrupt_manager = X86_64InterruptManager::new(memory.heap_allocator());
        interrupt_manager.bootstrap_local_interrupt_manager(local_interrupt_manager);

        Self {
            logger,
            timer,
            interrupt_manager,
            rsdp_address,
            application_execution_units: Mutex::new(BTreeMap::new_in(memory.heap_allocator())),
            memory,
            execution_unit,
        }
    }

    fn allocate_unprivileged_stack(&self) -> Option<u64> {
        Some(self.memory.allocate_user_stack()?.start().as_u64())
    }

    fn set_preemption_deadline(&self, duration: Duration) {
        self.interrupt_manager.reset_oneshot(duration);
    }

    fn clear_preemption_deadline(&self) {
        // An initial count of zero stops the one-shot timer.
        self.interrupt_manager.reset_oneshot(Duration::ZERO);
    }

    fn notify_end_of_interrupt(&self) {
        self.interrupt_manager.notify_local_end_of_interrupt();
    }

    fn execute_in_unprivileged(
        argument: *mut (),
        message: *const (),
        event: &mut Option<UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>>,
        stack: u64,
        entry: extern "C" fn(*mut (), *const ()) -> !,
    ) {
        x86_64_execute_in_unprivileged(argument, message, event, stack, entry);
    }

    fn resume_after_system_call(
        context: &Self::UnprivilegedSystemCallContext,
        event: &mut Option<UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>>,
    ) {
        x86_64_resume_after_system_call(context, event);
    }

    fn resume_after_preemption(
        context: &Self::UnprivilegedPreemptionContext,
        event: &mut Option<UnprivilegedEvent<Self::UnprivilegedSystemCallContext, Self::UnprivilegedPreemptionContext>>,
    ) {
        x86_64_resume_after_preemption(context, event);
    }

    fn complete_unprivileged_execution() -> ! {
        unsafe { core::arch::asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }
}
