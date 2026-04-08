use crate::actor::ActorRootEnvironmentMemoryService;
use crate::actor::ActorRootEnvironmentLoggerService;
use crate::actor::ActorRootEnvironmentTimerService;
use core::fmt::Write;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorRootEnvironmentArchitectureService {
    logger: ActorRootEnvironmentLoggerService,
    timer: ActorRootEnvironmentTimerService,
}

impl ActorRootEnvironmentArchitectureService {
    pub fn logger(&self) -> &ActorRootEnvironmentLoggerService {
        &self.logger
    }

    pub fn timer(&self) -> &ActorRootEnvironmentTimerService {
        &self.timer
    }

    pub fn new(
        boot_info: &'static mut bootloader_api::BootInfo,
    ) -> Self {
        let logger = crate::actor::ActorRootEnvironmentLoggerService::new(
            boot_info.framebuffer.take().map(|frame_buffer| {
                let info = frame_buffer.info().clone();

                bootloader_x86_64_common::framebuffer::FrameBufferWriter::new(frame_buffer.into_buffer(), info)
            }),
            Some(unsafe { bootloader_x86_64_common::serial::SerialPort::init() }),
        );


        let memory_manager = match ActorRootEnvironmentMemoryService::new(boot_info) {
            Ok(memory_manager) => memory_manager,
            Err(error) => {
                write!(logger.writer(), "{:?}", error);
                return todo!();
            }
        };

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
        use alloc::boxed::Box;

        let timer = ActorRootEnvironmentTimerService::new();

        let ring0_stack = memory_manager
            .allocate_stack()
            .unwrap()
            .initial_memory_address()
            .as_u64();

        let timer_stack = memory_manager
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
        )); // 8
        let kernel_data = gdt.append(Descriptor::UserSegment(DescriptorFlags::KERNEL_DATA.bits())); // 16
        // order is very important!
        let user_code32 = gdt.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE32.bits())); // 24
        let user_data = gdt.append(Descriptor::UserSegment(DescriptorFlags::USER_DATA.bits())); // 32
        let user_code64 = gdt.append(Descriptor::UserSegment(DescriptorFlags::USER_CODE64.bits())); // 40

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
            wrmsr(IA32_LSTAR, actor_system_call_entry_point as u64);
            wrmsr(IA32_FMASK, RFlags::FLAGS_IF.bits());
        }

        Box::into_raw(tss);
        Box::into_raw(gdt);

        /*let mut interrupt_manager = KernelInterruptManager::new();
        interrupt_manager.bootstrap_local_interrupt_manager({
            let mut local_interrupt_manager =
                crate::kernel::interrupt::LocalInterruptManager::new(
                    &timer,
                    &memory_manager.clone(),
                );

            local_interrupt_manager.enable_oneshot(
                unsafe {
                    core::mem::transmute(
                        actor_deadline_preemption_entry_point as *const u8,
                    )
                },
                core::time::Duration::from_millis(0),
            );

            local_interrupt_manager
        });*/

        Self {
            logger,
            timer,
        }
    }
}

use core::arch::naked_asm;

#[unsafe(naked)]
pub unsafe extern "C" fn actor_system_call_entry_point() -> ! {
    unsafe {
        naked_asm!(
            //
            // Store user context
            //
            "mov r9, rcx",
            "mov r10, rsp",
            //
            // Load kernel stack
            //
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",
            //
            // Restore
            //
            "mov rdx, r8",
            "pop rcx",
        )
    }
}

#[unsafe(naked)]
pub unsafe extern "C" fn actor_deadline_preemption_entry_point() -> ! {
    unsafe {
        naked_asm!(
            //
            // Store user context
            //
            "mov r9, rcx",
            "mov r10, rsp",
            //
            // Load kernel stack
            //
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",
            //
            // Restore
            //
            "mov rdx, r8",
            "pop rcx",
        )
    }
}
