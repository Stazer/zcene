use crate::common::logger::LoggerService;
use crate::platform::x86_64::X86_64MemoryService;
use crate::platform::x86_64::TimerService;
use crate::platform::Platform;
use core::fmt::Write;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct X86_64Platform {
    logger: LoggerService,
    memory: X86_64MemoryService,
    timer: TimerService,
}

impl X86_64Platform {
    pub fn logger(&self) -> &LoggerService {
        &self.logger
    }

    pub fn memory(&self) -> &X86_64MemoryService {
        &self.memory
    }

    pub fn timer(&self) -> &TimerService {
        &self.timer
    }
}

impl Platform for X86_64Platform {
    type BootInfo = &'static mut bootloader_api::BootInfo;

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

        let memory = match X86_64MemoryService::new(boot_info) {
            Ok(memory) => memory,
            Err(error) => {
                write!(logger.writer(), "{:?}", error);
                return todo!();
            }
        };

        use alloc::boxed::Box;
        use x86::current::rflags::RFlags;
        use x86::msr::{IA32_EFER, rdmsr, wrmsr};
        use x86::msr::{IA32_FMASK, IA32_LSTAR, IA32_STAR};
        use x86_64::VirtAddr;
        use x86_64::instructions::segmentation::Segment;
        use x86_64::instructions::tables::load_tss;
        use x86_64::registers::segmentation::CS;
        use x86_64::structures::gdt::GlobalDescriptorTable;
        use x86_64::structures::gdt::{Descriptor, DescriptorFlags};
        use x86_64::structures::tss::TaskStateSegment;

        let timer = TimerService::new();

        let ring0_stack = memory
            .allocate_stack()
            .unwrap()
            .initial_memory_address()
            .as_u64();

        let timer_stack = memory
            .allocate_stack()
            .unwrap()
            .initial_memory_address()
            .as_u64();

        unsafe {
            wrmsr(IA32_EFER, rdmsr(IA32_EFER) | 1);
        }

        let mut gdt = Box::new(GlobalDescriptorTable::new());

        let kernel_code = gdt.append(Descriptor::UserSegment(
            DescriptorFlags::KERNEL_CODE64.bits(),
        ));
        let kernel_data = gdt.append(Descriptor::UserSegment(DescriptorFlags::KERNEL_DATA.bits()));
        let user_code32 = gdt.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE32.bits()));
        let user_data = gdt.append(Descriptor::UserSegment(DescriptorFlags::USER_DATA.bits()));
        let user_code64 = gdt.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE64.bits()));

        let mut tss = Box::new(TaskStateSegment::new());
        tss.privilege_stack_table[0] = VirtAddr::new(ring0_stack);
        tss.interrupt_stack_table[0] = VirtAddr::new(timer_stack);
        tss.iomap_base = 0xFFFF;

        let tss_selector = unsafe {
            let descr = Descriptor::tss_segment_unchecked(Box::as_ptr(&tss));
            gdt.append(descr)
        };

        let selector = (u64::from(user_code32.0) << 48) | (u64::from(kernel_code.0) << 32);

        unsafe {
            gdt.load_unsafe();
            CS::set_reg(kernel_code);
            load_tss(tss_selector);
            wrmsr(IA32_STAR, selector);
            wrmsr(IA32_LSTAR, x86_64_system_call_entry_point as u64);
            wrmsr(IA32_FMASK, RFlags::FLAGS_IF.bits());
        }

        Box::into_raw(tss);
        Box::into_raw(gdt);

        Self { logger, memory, timer }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::arch::naked_asm;

#[unsafe(naked)]
pub unsafe extern "C" fn x86_64_system_call_entry_point() -> ! {
    unsafe {
        naked_asm!(
            "mov r9, rcx",
            "mov r10, rsp",
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",
            "mov rdx, r8",
            "pop rcx",
        )
    }
}

#[unsafe(naked)]
pub unsafe extern "C" fn x86_64_deadline_preemption_entry_point() -> ! {
    unsafe {
        naked_asm!(
            "mov r9, rcx",
            "mov r10, rsp",
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",
            "mov rdx, r8",
            "pop rcx",
        )
    }
}
