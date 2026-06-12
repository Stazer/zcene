use crate::common::logger::LoggerService;
use crate::platform::unprivileged_event::UnprivilegedEvent;
use crate::platform::x86_64::x86_64_unprivileged::{
    x86_64_execute_in_unprivileged, x86_64_resume_after_preemption, x86_64_resume_after_system_call,
};
use crate::platform::x86_64::{
    X86_64MemoryManager, X86_64UnprivilegedPreemptionContext, X86_64UnprivilegedSystemCallContext,
    TimerService,
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
    global_descriptor_table: Box<GlobalDescriptorTable, ErasedAllocator>,
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

        let memory = match X86_64MemoryManager::new(boot_info) {
            Ok(memory) => memory,
            Err(error) => {
                write!(logger.writer(), "{:?}", error);
                return todo!();
            }
        };

        let timer = TimerService::new();

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
        let kernel_data_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::KERNEL_DATA.bits()));
        let user_code32_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE32.bits()));
        let user_data_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::USER_DATA.bits()));
        let user_code64_segment_selector = global_descriptor_table.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE64.bits()));

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

        Self { logger, memory, timer, global_descriptor_table, task_state_segment }
    }

    fn allocate_unprivileged_stack(&self) -> Option<u64> {
        /*Some(
            self.memory
                .allocate_user_stack()?
                .start()
                .as_u64(),
    )*/

        todo!()
    }

    fn set_preemption_deadline(&self, _duration: Duration) {
        todo!("interrupt manager not yet wired into X86_64Platform")
    }

    fn notify_end_of_interrupt(&self) {
        todo!("interrupt manager not yet wired into X86_64Platform")
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
