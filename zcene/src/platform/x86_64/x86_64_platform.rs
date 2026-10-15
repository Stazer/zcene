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
use x86_64::structures::gdt::GlobalDescriptorTable;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags};
use x86_64::structures::tss::TaskStateSegment;
use core::time::Duration;
use alloc::boxed::Box;
use x86::current::rflags::RFlags;
use crate::platform::x86_64::x86_64_system_call_entry_point;
use x86::msr::{IA32_EFER, rdmsr, wrmsr};
use x86::msr::{IA32_FMASK, IA32_LSTAR, IA32_STAR};
use x86_64::VirtAddr;
use x86_64::instructions::segmentation::Segment;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::CS;
use crate::common::memory::allocator::ErasedAllocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct X86_64Platform {
    logger: LoggerService,
    memory: X86_64MemoryManager,
    timer: TimerService,
    interrupt_manager: X86_64InterruptManager,
    rsdp_address: Option<u64>,
    // Kept alive: the GDT/TSS registers hold raw pointers into these allocations.
    #[allow(dead_code)]
    global_descriptor_table: Box<GlobalDescriptorTable, ErasedAllocator>,
    #[allow(dead_code)]
    task_state_segment: Box<TaskStateSegment, ErasedAllocator>,
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

        let ring0_stack = memory
            .allocate_stack()
            .unwrap()
            .start()
            .as_u64();

        let timer_stack = memory
            .allocate_stack()
            .unwrap()
            .start()
            .as_u64();

        unsafe {
            wrmsr(IA32_EFER, rdmsr(IA32_EFER) | 1);
        }

        let mut global_descriptor_table = Box::new_in(GlobalDescriptorTable::new(), memory.heap_allocator());
        let kernel_code_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::KERNEL_CODE64.bits()));
        let _kernel_data_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::KERNEL_DATA.bits()));
        let user_code32_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE32.bits()));
        let _user_data_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::USER_DATA.bits()));
        let _user_code64_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE64.bits()));

        let mut task_state_segment = Box::new_in(TaskStateSegment::new(), memory.heap_allocator());
        task_state_segment.privilege_stack_table[0] = VirtAddr::new(ring0_stack);
        task_state_segment.interrupt_stack_table[0] = VirtAddr::new(timer_stack);
        task_state_segment.iomap_base = 0xFFFF;

        let task_state_segment_selector = global_descriptor_table.append(
            unsafe {
                Descriptor::tss_segment_unchecked(&*task_state_segment)
            }
        );
        let ia32_star_selector =
            (u64::from(user_code32_segment_selector.0) << 48)
            | (u64::from(kernel_code_segment_selector.0) << 32);

        unsafe {
            global_descriptor_table.load_unsafe();
            CS::set_reg(kernel_code_segment_selector);
            load_tss(task_state_segment_selector);
            wrmsr(IA32_STAR, ia32_star_selector);
            wrmsr(IA32_LSTAR, x86_64_system_call_entry_point as *const () as u64);
            wrmsr(IA32_FMASK, RFlags::FLAGS_IF.bits());
        }

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
        local_interrupt_manager
            .enable_oneshot(deadline_preemption_entry_point, Duration::from_millis(1));

        let mut interrupt_manager = X86_64InterruptManager::new(memory.heap_allocator());
        interrupt_manager.bootstrap_local_interrupt_manager(local_interrupt_manager);

        Self {
            logger,
            memory,
            timer,
            interrupt_manager,
            rsdp_address,
            global_descriptor_table,
            task_state_segment,
        }
    }

    fn allocate_unprivileged_stack(&self) -> Option<u64> {
        Some(self.memory.allocate_user_stack()?.start().as_u64())
    }

    fn set_preemption_deadline(&self, duration: Duration) {
        self.interrupt_manager.reset_oneshot(duration);
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
