use crate::kernel::actor::{
    KernelActorAddressReference, KernelActorHandler, KernelActorSystem, KernelActorSystemReference,
};
use crate::kernel::future::runtime::{KernelFutureRuntime, KernelFutureRuntimeHandler};
use crate::kernel::memory::{InitializeMemoryManagerError, KernelMemoryManager};
use core::mem::MaybeUninit;
use crate::kernel::interrupt::KernelInterruptManager;
use bootloader_x86_64_common::serial::SerialPort;
use crate::kernel::KernelTimer;
use alloc::alloc::Global;
use alloc::sync::Arc;
use crate::kernel::logger::KernelLogger;
use bootloader_api::BootInfo;
use core::cell::SyncUnsafeCell;
use core::fmt::{self, Write};
use x86::cpuid::CpuId;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::frame::FrameManagerAllocationError;
use zcene_core::actor::ActorAddressExt;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use zcene_core::actor::{
    Actor, ActorFuture, ActorHandleError, ActorHandler,
};
use zcene_core::future::FutureExt;

////////////////////////////////////////////////////////////////////////////////////////////////////

const FRAME_SIZE: usize = 4096;
const EXTERNAL_INTERRUPTS_START: usize = 0x20;
const TIMER_INTERRUPT_ID: usize = 0x20;
const SPURIOUS_ID: usize = 0x20 + 15;
const KEYBOARD_INTERRUPT_ID: usize = 0x21;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum KernelInitializeError {
    FrameBufferUnavailable,
    PhysicalMemoryOffsetUnvailable,
    FrameAllocation(FrameManagerAllocationError),
    Fmt(fmt::Error),
    BuildLocalApic(&'static str),
    InitializeMemoryManager(InitializeMemoryManagerError),
}

impl From<fmt::Error> for KernelInitializeError {
    fn from(error: fmt::Error) -> Self {
        Self::Fmt(error)
    }
}

impl From<FrameManagerAllocationError> for KernelInitializeError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl From<InitializeMemoryManagerError> for KernelInitializeError {
    fn from(error: InitializeMemoryManagerError) -> Self {
        Self::InitializeMemoryManager(error)
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

            crate::common::println!(
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

#[derive(Constructor, Default)]
pub struct RootActor {
    subscriptions: Vec<ActorMailbox<(), KernelActorHandler>, Global>,
}

#[derive(Clone)]
pub enum RootActorMessage {
    Subscription(ActorMailbox<(), KernelActorHandler>),
    NoOperation,
}

impl<H> Actor<H> for RootActor
where
    H: ActorHandler,
    H::HandleContext<RootActorMessage>: ActorContextMessageProvider<RootActorMessage>,
{
    type Message = RootActorMessage;

    fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
            match context.message() {
                RootActorMessage::Subscription(subscription) => {
                    let subscription = subscription.clone();

                    subscription.send(()).await.unwrap();

                    self.subscriptions.push(subscription);
                }
                RootActorMessage::NoOperation => {}
            };

            Ok(())
        }
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

    fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
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

        Kernel::get().run();

        loop {}
    }

    pub fn application_processor_entry_point() -> ! {
        loop {}
    }

    pub fn new(boot_info: &'static mut BootInfo) -> Result<Self, KernelInitializeError> {
        let logger = KernelLogger::new(
            boot_info.framebuffer.take().map(|frame_buffer| {
                let info = frame_buffer.info().clone();

                FrameBufferWriter::new(frame_buffer.into_buffer(), info)
            }),
            Some(unsafe { SerialPort::init() }),
        );

        let memory_manager = KernelMemoryManager::new(boot_info)?;

        let actor_system = KernelActorSystem::try_new(KernelActorHandler::new(
            KernelFutureRuntime::new(KernelFutureRuntimeHandler::default()).unwrap(),
            Arc::default(),
        ))
        .unwrap();

        let timer = KernelTimer::new(
            &memory_manager,
            boot_info.rsdp_addr.into_option().map(PhysicalMemoryAddress::from),
        );

        let mut interrupt_manager = KernelInterruptManager::new();
        interrupt_manager.bootstrap_local_interrupt_manager(
            {
                let mut local_interrupt_manager = crate::kernel::interrupt::LocalInterruptManager::new(&memory_manager);
                local_interrupt_manager.enable_timer(
                    &timer,
                    unsafe {
                    core::mem::transmute(crate::kernel::actor::timer_entry_point as *const u8)// as *const u8 as crate::kernel::interrupt::InterruptEntryPoint,
                    },
                    core::time::Duration::from_millis(100),
                );

                local_interrupt_manager
            }
        );

        let timer_actor = actor_system.spawn(TimerActor::default()).unwrap();

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
        self.timer_actor
            .send(TimerActorMessage::Subscription(
                self.actor_system
                    .spawn(ApplicationActor::new(1, 0))
                    .unwrap()
                    .mailbox()
                    .unwrap(),
            ))
            .complete()
            .unwrap();

        self.timer_actor
            .send(TimerActorMessage::Subscription(
                self.actor_system
                    .spawn(ApplicationActor::new(2, 0))
                    .unwrap()
                    .mailbox()
                    .unwrap(),
            ))
            .complete()
            .unwrap();

        self.timer_actor
            .send(TimerActorMessage::Subscription(
                self.actor_system
                    .spawn(ApplicationActor::new(3, 0))
                    .unwrap()
                    .mailbox()
                    .unwrap(),
            ))
            .complete()
            .unwrap();

        self.logger().writer(|w| write!(w, "zcene\n",));

        //self.initialize_interrupts();

        x86_64::instructions::interrupts::enable();

        self.actor_system().enter().unwrap();

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

entry_point!(Kernel::bootstrap_processor_entry_point, config = &BOOTLOADER_CONFIG);
