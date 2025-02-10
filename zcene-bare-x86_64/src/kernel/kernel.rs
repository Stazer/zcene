use crate::kernel::actor::{
    KernelActorAddressReference, KernelActorHandler, KernelActorSystem, KernelActorSystemReference,
};
use crate::kernel::future::runtime::{KernelFutureRuntime, KernelFutureRuntimeHandler};
use crate::kernel::interrupt::KernelInterruptManager;
use crate::kernel::logger::println;
use crate::kernel::logger::KernelLogger;
use crate::kernel::memory::{KernelMemoryManager, KernelMemoryManagerInitializeError};
use crate::kernel::KernelTimer;
use alloc::alloc::Global;
use alloc::sync::Arc;
use bootloader_api::BootInfo;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use bootloader_x86_64_common::serial::SerialPort;
use core::cell::SyncUnsafeCell;
use core::fmt::{self, Write};
use core::mem::MaybeUninit;
use x86::cpuid::CpuId;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::frame::FrameManagerAllocationError;
use zcene_core::actor::ActorAddressExt;
use zcene_core::actor::ActorSpawnError;
use zcene_core::actor::{Actor, ActorFuture, ActorHandleError, ActorHandler};
use zcene_core::future::FutureExt;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum KernelInitializeError {
    FrameBufferUnavailable,
    PhysicalMemoryOffsetUnvailable,
    FrameAllocation(FrameManagerAllocationError),
    Fmt(fmt::Error),
    BuildLocalApic(&'static str),
    KernelMemoryManagerInitialize(KernelMemoryManagerInitializeError),
    ActorSpawn(ActorSpawnError),
}

impl From<fmt::Error> for KernelInitializeError {
    fn from(error: fmt::Error) -> Self {
        Self::Fmt(error)
    }
}

impl From<ActorSpawnError> for KernelInitializeError {
    fn from(error: ActorSpawnError) -> Self {
        Self::ActorSpawn(error)
    }
}

impl From<FrameManagerAllocationError> for KernelInitializeError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl From<KernelMemoryManagerInitializeError> for KernelInitializeError {
    fn from(error: KernelMemoryManagerInitializeError) -> Self {
        Self::KernelMemoryManagerInitialize(error)
    }
}

#[derive(Constructor, Default)]
pub struct ApplicationActor {
    number: usize,
    times: usize,
}

impl<H> Actor<H> for ApplicationActor
where
    H: ActorHandler,
    H::HandleContext<usize>: ActorContextMessageProvider<usize>,
{
    type Message = usize;

    fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
            self.times += 1;

            let cpu_id = CpuId::new();
            let feature_info = cpu_id.get_feature_info().unwrap();

            println!(
                "application #{}, times: {}, ticks: {}, CPU: {}",
                self.number,
                self.times,
                context.message(),
                feature_info.initial_local_apic_id()
            );

            Ok(())
        }
    }
}

use core::marker::PhantomData;
use zcene_core::actor::{ActorCreateError, ActorDestroyError};

pub struct UnprivilegedActor<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    handler: PhantomData<H>,
}

impl<A, H> Actor<H> for UnprivilegedActor<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    type Message = ();

    async fn create(&mut self, context: H::CreateContext) -> Result<(), ActorCreateError> {
        Ok(())
    }

    async fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        Ok(())
    }

    async fn destroy(self, context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

use alloc::vec::Vec;
use zcene_core::actor::ActorContextMessageProvider;
use zcene_core::actor::{ActorMailbox, ActorMessageSender};
use ztd::Constructor;

#[derive(Default, Constructor)]
pub struct TimerActor {
    subscriptions: Vec<ActorMailbox<usize, KernelActorHandler>>,
    total_ticks: usize,
}

#[derive(Clone)]
pub enum TimerActorMessage {
    Tick,
    Subscription(ActorMailbox<usize, KernelActorHandler>),
}

impl<H> Actor<H> for TimerActor
where
    H: ActorHandler,
    H::HandleContext<TimerActorMessage>: ActorContextMessageProvider<TimerActorMessage>,
{
    type Message = TimerActorMessage;

    fn create(
        &mut self,
        _context: H::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        async move {
            Ok(())
        }
    }

    fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
            println!("HELLO");

            match context.message() {
                Self::Message::Tick => {
                    self.total_ticks += 1;

                    for subscription in &self.subscriptions {
                        subscription.send(self.total_ticks).await.unwrap();
                    }
                }
                Self::Message::Subscription(mailbox) => {
                    self.subscriptions.push(mailbox.clone());
                }
            }

            Ok(())
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Kernel {
    logger: KernelLogger,
    memory_manager: KernelMemoryManager,
    actor_system: KernelActorSystemReference,
    interrupt_manager: KernelInterruptManager,
    timer: KernelTimer<'static>,
    timer_actor: KernelActorAddressReference<TimerActor>,
}

impl Kernel {
    pub fn bootstrap_processor_entry_point(boot_info: &'static mut BootInfo) -> ! {
        unsafe {
            KERNEL
                .get()
                .as_mut()
                .unwrap()
                .write(Kernel::new(boot_info).unwrap());
        }

        unsafe {
            use x86::msr::{rdmsr, wrmsr, IA32_EFER};

            wrmsr(IA32_EFER, rdmsr(IA32_EFER) | 1);
        }

        use crate::kernel::actor::KernelActorExecutionMode;
        use crate::kernel::actor::KernelActorInstructionRegion;
        use crate::kernel::actor::KernelActorSpawnSpecification;
        use crate::user::MainActor;
        use zcene_bare::memory::address::VirtualMemoryAddress;

        use crate::user::USER_SECTIONS_SIZE;
        use crate::user::USER_SECTIONS_START;

        /*println!(
            "{} {} {}",
            <MainActor as Actor<KernelActorHandler>>::create as u64,
            <MainActor as Actor<KernelActorHandler>>::handle as u64,
            <MainActor as Actor<KernelActorHandler>>::destroy as u64
        );*/

        Kernel::get()
            .actor_system()
            .spawn(KernelActorSpawnSpecification::new(
                MainActor,
                KernelActorExecutionMode::Unprivileged,
                KernelActorInstructionRegion::new(
                    VirtualMemoryAddress::new(
                        Kernel::get()
                            .memory_manager()
                            .kernel_image_virtual_memory_region()
                            .start()
                            .as_usize()
                            + unsafe { linker_value(&USER_SECTIONS_START) },
                    ),
                    unsafe { linker_value(&USER_SECTIONS_SIZE) },
                ),
            ));

        use zcene_bare::common::linker_value;

        Kernel::get().run();

        loop {}
    }

    pub fn application_processor_entry_point() -> ! {
        loop {}
    }

    #[naked]
    pub unsafe fn system_call() {
        core::arch::naked_asm!("nop",)
    }

    pub fn new(boot_info: &'static mut BootInfo) -> Result<Self, KernelInitializeError> {
        let logger = KernelLogger::new(
            boot_info.framebuffer.take().map(|frame_buffer| {
                let info = frame_buffer.info().clone();

                FrameBufferWriter::new(frame_buffer.into_buffer(), info)
            }),
            Some(unsafe { SerialPort::init() }),
        );

        let memory_manager = match KernelMemoryManager::new(&logger, boot_info) {
            Ok(memory_manager) => memory_manager,
            Err(error) => {
                logger.writer(|writer| write!(writer, "{:?}", error));
                return Err(error.into());
            }
        };

        let actor_system = KernelActorSystem::try_new(KernelActorHandler::new(
            KernelFutureRuntime::new(KernelFutureRuntimeHandler::default()).unwrap(),
            Arc::default(),
            zcene_bare::synchronization::Mutex::default(),
        ))
        .unwrap();

        let timer = KernelTimer::new(
            &memory_manager,
            boot_info
                .rsdp_addr
                .into_option()
                .map(PhysicalMemoryAddress::from),
        );

        let mut interrupt_manager = KernelInterruptManager::new();
        interrupt_manager.bootstrap_local_interrupt_manager({
            let mut local_interrupt_manager =
                crate::kernel::interrupt::LocalInterruptManager::new(&memory_manager);
            local_interrupt_manager.enable_timer(
                &timer,
                unsafe {
                    core::mem::transmute(crate::kernel::actor::timer_entry_point as *const u8)
                },
                core::time::Duration::from_millis(100),
            );

            local_interrupt_manager
        });

        use crate::kernel::actor::KernelActorExecutionMode;
        use crate::kernel::actor::KernelActorInstructionRegion;
        use crate::kernel::actor::KernelActorSpawnSpecification;
        use zcene_bare::memory::address::VirtualMemoryAddress;

        let spawn_result = actor_system.spawn(KernelActorSpawnSpecification::new(
            TimerActor::default(),
            KernelActorExecutionMode::Privileged,
            memory_manager.kernel_image_virtual_memory_region(),
        ));

        let timer_actor = match spawn_result {
            Ok(timer_actor) => timer_actor,
            Err(error) => {
                logger.writer(|w| write!(w, "Error {:?}\n", error));

                return Err(error.into());
            }
        };

        let this = Self {
            logger,
            actor_system,
            timer_actor,
            memory_manager,
            timer,
            interrupt_manager,
        };

        Ok(this)
    }

    pub fn get<'a>() -> &'a Kernel {
        unsafe { KERNEL.get().as_ref().unwrap().as_ptr().as_ref().unwrap() }
    }

    pub fn memory_manager(&self) -> &KernelMemoryManager {
        &self.memory_manager
    }

    pub fn timer_actor(&self) -> &KernelActorAddressReference<TimerActor> {
        &self.timer_actor
    }

    pub fn logger(&self) -> &KernelLogger {
        &self.logger
    }

    pub fn actor_system(&self) -> &KernelActorSystemReference {
        &self.actor_system
    }

    pub fn timer(&self) -> &KernelTimer {
        &self.timer
    }

    pub fn interrupt_manager(&self) -> &KernelInterruptManager {
        &self.interrupt_manager
    }

    pub fn run(&self) -> ! {
        self.logger().writer(|w| write!(w, "zcene\n"));

        x86_64::instructions::interrupts::enable();

        self.actor_system().enter_default().unwrap();

        loop {}
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub static KERNEL: SyncUnsafeCell<MaybeUninit<Kernel>> = SyncUnsafeCell::new(MaybeUninit::zeroed());

////////////////////////////////////////////////////////////////////////////////////////////////////

use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootloaderConfig};

////////////////////////////////////////////////////////////////////////////////////////////////////

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = 4 * 4096;
    config.mappings.physical_memory = Some(Mapping::Dynamic);

    config
};

////////////////////////////////////////////////////////////////////////////////////////////////////

entry_point!(
    Kernel::bootstrap_processor_entry_point,
    config = &BOOTLOADER_CONFIG
);
