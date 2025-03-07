use crate::kernel::future::runtime::{KernelFutureRuntime, KernelFutureRuntimeHandler};
use crate::kernel::interrupt::KernelInterruptManager;
use crate::kernel::logger::println;
use crate::kernel::logger::KernelLogger;
use crate::kernel::memory::{KernelMemoryManager, KernelMemoryManagerInitializeError};
use crate::kernel::KernelTimer;
use bootloader_api::BootInfo;
use bootloader_x86_64_common::framebuffer::FrameBufferWriter;
use bootloader_x86_64_common::serial::SerialPort;
use core::cell::SyncUnsafeCell;
use core::fmt::{self, Write};
use core::mem::MaybeUninit;
use crate::memory::address::PhysicalMemoryAddress;
use crate::memory::frame::FrameManagerAllocationError;
use zcene_core::actor::ActorEnvironment;
use zcene_core::actor::ActorSpawnError;
use zcene_core::actor::{self, Actor, ActorHandleError, ActorSystemCreateError};
use zcene_core::actor::{ActorCreateError, ActorDestroyError, ActorMessage, ActorMessageSender};
use zcene_core::future::runtime::FutureRuntimeCreateError;
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
    ActorSystemCreate(ActorSystemCreateError),
}

#[derive(Debug, Constructor, Default)]
pub struct PrintActor;

#[derive(Debug, Constructor, Clone)]
pub struct PrintActorMessage {
    value0: usize,
    value1: usize,
}

impl<H> Actor<H> for PrintActor
where
    H: actor::ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    type Message = PrintActorMessage;

    async fn create(&mut self, _context: H::CreateContext) -> Result<(), ActorCreateError> {
        println!("create");
        Ok(())
    }

    async fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        println!("Received {:?}", context.message());

        Ok(())
    }

    async fn destroy(self, _context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        println!("destroy");
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug)]
pub struct UnprivilegedActor<H>
where
    H: ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    printer: H::Address<PrintActor>,
}

impl<H> Actor<H> for UnprivilegedActor<H>
where
    H: ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
    H::HandleContext<usize>: ActorContextMessageProvider<usize>,
{
    type Message = usize;

    async fn create(&mut self, _context: H::CreateContext) -> Result<(), ActorCreateError> {
        self.printer.send(PrintActorMessage::new(1337, 1338)).await;

        Ok(())
    }

    async fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        self.printer
            .send(PrintActorMessage::new(context.message() + 1, 1338))
            .await;

        Ok(())
    }

    async fn destroy(self, _context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct LastActor<H>
where
    H: ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
    H::HandleContext<usize>: ActorContextMessageProvider<usize>,
{
    unpriv: H::Address<UnprivilegedActor<H>>,
}

impl<H> Actor<H> for LastActor<H>
where
    H: ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
    H::HandleContext<usize>: ActorContextMessageProvider<usize>,
{
    type Message = ();

    async fn create(&mut self, _context: H::CreateContext) -> Result<(), ActorCreateError> {
        self.unpriv.send(42).await;

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

use crate::actor::ActorRootEnvironment;
use zcene_core::actor::ActorContextMessageProvider;
use zcene_core::actor::{ActorSystem, ActorSystemReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Kernel {
    logger: KernelLogger,
    memory_manager: KernelMemoryManager,
    actor_system: ActorSystemReference<ActorRootEnvironment<KernelFutureRuntimeHandler>>,
    interrupt_manager: KernelInterruptManager,
    timer: KernelTimer<'static>,
    //timer_actor: KernelActorAddressReference<TimerActor>,
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

        use crate::actor::{
            ActorIsolationAddress, ActorIsolationEnvironment, ActorIsolationSpawnSpecification,
            ActorRootEnvironment, ActorRootSpawnSpecification,
        };

        use crate::actor::ActorIsolationMessageHandler;
        use alloc::boxed::Box;
        use alloc::vec;

        use zcene_core::actor::ActorMessageChannelAddress;

        let print_actor_address: ActorMessageChannelAddress<PrintActor, ActorRootEnvironment<_>> =
            Kernel::get()
                .actor_system()
                .spawn(ActorRootSpawnSpecification::new(
                    PrintActor::default(),
                    None,
                ))
                .unwrap();

        let unpriv_actor = Kernel::get()
            .actor_system()
            .spawn(ActorIsolationSpawnSpecification::<
                UnprivilegedActor<_>,
                UnprivilegedActor<_>,
                _,
            >::new(
                UnprivilegedActor::<ActorIsolationEnvironment>::new(ActorIsolationAddress::<
                    PrintActor,
                >::new(0)),
                None,
                vec![Box::<dyn ActorIsolationMessageHandler<_>>::from(
                    Box::new_in(
                        print_actor_address.clone(),
                        *Kernel::get().actor_system().allocator(),
                    ),
                )],
            ))
            .unwrap();

        let _last_actor = Kernel::get()
            .actor_system()
            .spawn(ActorRootSpawnSpecification::new(
                LastActor::new(unpriv_actor),
                None,
            ))
            .unwrap();

        Kernel::get().run();

        loop {}
    }

    pub fn application_processor_entry_point() -> ! {
        loop {}
    }

    pub fn new(boot_info: &'static mut BootInfo) -> Result<Self, KernelInitializeError> {
        let mut logger = KernelLogger::new(
            boot_info.framebuffer.take().map(|frame_buffer| {
                let info = frame_buffer.info().clone();

                FrameBufferWriter::new(frame_buffer.into_buffer(), info)
            }),
            Some(unsafe { SerialPort::init() }),
        );

        let memory_manager = match KernelMemoryManager::new(boot_info, &mut logger) {
            Ok(memory_manager) => memory_manager,
            Err(error) => {
                logger.writer(|writer| write!(writer, "{:?}", error));
                return Err(error.into());
            }
        };

        logger.writer(|writer| write!(writer, "hello"));

        let actor_system = ActorSystem::try_new(ActorRootEnvironment::new(
            KernelFutureRuntime::new(KernelFutureRuntimeHandler::default())?,
        ))?;

        let timer = KernelTimer::new(
            &memory_manager,
            boot_info
                .rsdp_addr
                .into_option()
                .map(PhysicalMemoryAddress::from),
        );

        use crate::actor::actor_system_call_entry_point;
        use alloc::boxed::Box;
        use x86::current::rflags::RFlags;
        use x86::msr::{rdmsr, wrmsr, IA32_EFER};
        use x86::msr::{IA32_FMASK, IA32_LSTAR, IA32_STAR};
        use x86_64::instructions::segmentation::Segment;
        use x86_64::instructions::tables::load_tss;
        use x86_64::registers::segmentation::CS;
        use x86_64::structures::gdt::GlobalDescriptorTable;
        use x86_64::structures::gdt::{Descriptor, DescriptorFlags};
        use x86_64::structures::tss::TaskStateSegment;
        use x86_64::VirtAddr;

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

        /*logger.writer(|w| {
            write!(
                w,
                "{:X?} {:X?} {:X?} {:X?} {:X?}\n",
                ring0_stack,
                gdt,
                tss,
                [kernel_code, kernel_data, user_code, user_data, tss_selector,],
                [kernel_code.0, kernel_data.0, user_code.0, user_data.0,]
            )
        });*/

        Box::into_raw(tss);

        Box::into_raw(gdt);

        /*logger.writer(|w| {
            write!(
                w,
                "{:X?} {:X?}\n",
                DescriptorFlags::USER_CODE64,
                DescriptorFlags::USER_DATA
            )
        });*/

        let mut interrupt_manager = KernelInterruptManager::new();
        interrupt_manager.bootstrap_local_interrupt_manager({
            let mut local_interrupt_manager =
                crate::kernel::interrupt::LocalInterruptManager::new(&timer, &memory_manager);

            local_interrupt_manager.enable_oneshot(
                unsafe {
                    core::mem::transmute(
                        crate::actor::actor_deadline_preemption_entry_point as *const u8,
                    )
                },
                core::time::Duration::from_millis(0),
                &logger,
            );

            local_interrupt_manager
        });

        let this = Self {
            logger,
            actor_system,
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

    pub fn logger(&self) -> &KernelLogger {
        &self.logger
    }

    pub fn actor_system(
        &self,
    ) -> &ActorSystemReference<ActorRootEnvironment<KernelFutureRuntimeHandler>> {
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

entry_point!(
    Kernel::bootstrap_processor_entry_point,
    config = &BOOTLOADER_CONFIG
);
