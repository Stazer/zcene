use alloc::alloc::Global;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use bootloader_api::BootInfo;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use bootloader_x86_64_common::serial::SerialPort;
use core::alloc::AllocError;
use core::fmt::{self, Write};
use crate::actor::ActorRootEnvironment;
use zcene_core::future::runtime::{FutureRuntime, FutureRuntimeHandler};
use crate::kernel::KernelTimer;
use crate::kernel::interrupt::KernelInterruptManager;
use crate::kernel::memory::{KernelMemoryManager, KernelMemoryManagerInitializeError};
use crate::memory::address::PhysicalMemoryAddress;
use crate::memory::allocator::FrameManagerAllocationError;
use crate::{ACTOR_ROOT_ENVIRONMENT};
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
use zcene_core::actor::ActorEnvironment;
use zcene_core::actor::ActorMessageChannelAddress;
use zcene_core::actor::ActorSpawnError;
use zcene_core::actor::{ActorEnvironmentReference};
use zcene_core::actor::{ActorEnvironmentSpawn, ActorEnvironmentEnterable, ActorCreateError, ActorDestroyError, ActorMessage, ActorMessageSender};
use zcene_core::actor::{self, Actor, ActorHandleError, ActorSystemCreateError};
use zcene_core::future::runtime::{FutureRuntimeReference, FutureRuntimeCreateError};
use ztd::Constructor;
use ztd::From;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, From)]
#[From(unnamed)]
pub enum KernelInitializeError {
    FrameBufferUnavailable,
    PhysicalMemoryOffsetUnvailable,
    FrameAllocation(FrameManagerAllocationError),
    Fmt(fmt::Error),
    BuildLocalApic(&'static str),
    KernelMemoryManagerInitialize(KernelMemoryManagerInitializeError),
    ActorSpawn(ActorSpawnError),
    FutureRuntimeCreate(FutureRuntimeCreateError),
    Allocation(AllocError),
    ActorSystemCreate(ActorSystemCreateError),
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Kernel;

impl Kernel {
    pub fn bootstrap_processor_entry_point<A>(
        boot_info: &'static mut BootInfo,
        actor: A,
    ) -> !
    where
        A: Actor<ActorRootEnvironment>,
    {
        let environment = Self::new(
            boot_info,
        ).unwrap();

        unsafe {
            ACTOR_ROOT_ENVIRONMENT
                .get()
                .as_mut()
                .unwrap()
                .write(environment.into());
        }

        let address = crate::actor::ActorRootEnvironment::get()
            .spawn(actor)
            .unwrap();

        crate::actor::ActorRootEnvironment::get().enter();

        loop {}
    }

    pub fn application_processor_entry_point() -> ! {
        loop {}
    }

    pub fn new(boot_info: &'static mut BootInfo) -> Result<zcene_core::actor::ActorEnvironmentReference<crate::actor::ActorRootEnvironment>, KernelInitializeError>
    {
        let logger = crate::actor::ActorRootEnvironmentLoggerService::new(
            boot_info.framebuffer.take().map(|frame_buffer| {
                let info = frame_buffer.info().clone();

                FrameBufferWriter::new(frame_buffer.into_buffer(), info)
            }),
            Some(unsafe { SerialPort::init() }),
        );

        let memory_manager = match KernelMemoryManager::new(boot_info) {
            Ok(memory_manager) => memory_manager,
            Err(error) => {
                write!(logger.writer(), "{:?}", error);
                return Err(error.into());
            }
        };

        let timer = KernelTimer::new(
            &memory_manager,
            boot_info
                .rsdp_addr
                .into_option()
                .map(PhysicalMemoryAddress::from),
        );

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

        let mut interrupt_manager = KernelInterruptManager::new();
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
        });

        let actor_system = ActorEnvironmentReference::new(
            crate::actor::ActorRootEnvironment::new(
                FutureRuntime::new(
                    crate::future::runtime::FutureRuntimeHandler::default()
                ).unwrap(),
                logger,
                timer,
                memory_manager,
                interrupt_manager,
            ),
        );

        Ok(actor_system)
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
