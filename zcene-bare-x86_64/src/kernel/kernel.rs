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
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use x86::cpuid::CpuId;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::frame::FrameManagerAllocationError;
use zcene_core::actor::ActorSpawnError;
use zcene_core::actor::{self, Actor, ActorFuture, ActorHandleError, ActorSystemCreateError};
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

pub type PrintActorMessage = usize;

impl<H> Actor<H> for PrintActor
where
    H: actor::ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    type Message = PrintActorMessage;

    async fn create(&mut self, context: H::CreateContext) -> Result<(), ActorCreateError> {
        println!("zup");
        Ok(())
    }

    async fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        println!("Received {}", context.message());

        Ok(())
    }

    async fn destroy(self, context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct EmptyActor;

impl<H> Actor<H> for EmptyActor
where
    H: actor::ActorEnvironment,
{
    type Message = ();

    async fn create(&mut self, context: H::CreateContext) -> Result<(), ActorCreateError> {
        Ok(())
    }

    async fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        Ok(())
    }

    async fn destroy(self, context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        Ok(())
    }
}

#[derive(Debug)]
// #[derive(ActorEnvironmentTransformer(ActorUnprivilegedEnvironment))]
pub struct UnprivilegedActor<H>
where
    H: actor::ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    printer: H::Address<PrintActor>,
}

impl<H> UnprivilegedActor<H>
where
    H: actor::ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    pub fn new(printer: H::Address<PrintActor>) -> Self {
        Self { printer }
    }
}

impl<H> Actor<H> for UnprivilegedActor<H>
where
    H: actor::ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    type Message = ();

    async fn create(&mut self, context: H::CreateContext) -> Result<(), ActorCreateError> {
        self.printer.send(55).await;

        Ok(())
    }

    async fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> Result<(), ActorHandleError> {
        Ok(())
    }

    async fn destroy(self, context: H::DestroyContext) -> Result<(), ActorDestroyError> {
        Ok(())
    }
}

use crate::actor::{
    ActorEnvironmentTransformer, ActorUnprivilegedAddress, ActorUnprivilegedHandler,
};
use zcene_core::actor::ActorEnvironment;

impl<H> ActorEnvironmentTransformer<ActorUnprivilegedHandler> for UnprivilegedActor<H>
where
    H: ActorEnvironment,
    H::HandleContext<PrintActorMessage>: ActorContextMessageProvider<PrintActorMessage>,
{
    type Output = UnprivilegedActor<ActorUnprivilegedHandler>;

    fn transform(self) -> Self::Output {
        Self::Output::new(ActorUnprivilegedAddress::new(0))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

use crate::actor::ActorHandler;
use zcene_core::actor::ActorContextMessageProvider;
use zcene_core::actor::{ActorSystem, ActorSystemReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Kernel {
    logger: KernelLogger,
    memory_manager: KernelMemoryManager,
    actor_system: ActorSystemReference<ActorHandler<KernelFutureRuntimeHandler>>,
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

        unsafe {
            use x86::msr::{rdmsr, wrmsr, IA32_EFER};

            wrmsr(IA32_EFER, rdmsr(IA32_EFER) | 1);
        }

        use core::num::NonZero;

        use crate::actor::{
            ActorPrivilegedHandlerSpawnSpecification, ActorUnprivilegedHandlerSpawnSpecification,
        };

        let print_actor = Kernel::get()
            .actor_system()
            .spawn(ActorPrivilegedHandlerSpawnSpecification::new(
                PrintActor::default(),
                None,
            ))
            .unwrap();

        let address =
            Kernel::get()
                .actor_system()
                .spawn(ActorUnprivilegedHandlerSpawnSpecification::new(
                    UnprivilegedActor::new(print_actor.clone()),
                    None,
                ));

        /*Kernel::get()
        .actor_system()
        .spawn(ActorSpawnSpecification::new(
            UnprivilegedActor::new(print_actor),
            ActorUnprivilegedSpawnSpecification::new(NonZero::new(100)).into(),
        ));*/

        /*Kernel::get()
        .actor_system()
        .spawn(PrintActor::default());*/

        use crate::actor::ActorUnprivilegedHandler;

        /*let print_address = Kernel::get()
            .actor_system()
            .handler()
            .spawn_custom(PrintActor);

        let address = Kernel::get()
            .actor_system()
            .handler()
            .spawn_custom(ActorUnprivilegedHandlerSpawnSpecification{
                actor: UnprivilegedActor::new(print_address),
                addresses: Vec::default(),
                deadline_in_milliseconds: None,
                marker: PhantomData::<_>,
            });*/

        /*Kernel::get()
        .actor_system()
        .spawn::<_, ActorUnprivilegedHandler>(
            EmptyActor,
        ).unwrap();*/

        //use alloc::vec::Vec;
        //use crate::actor::{ActorUnprivilegedAddress, ActorUnprivilegedHandlerSpawnSpecification};

        /*Kernel::get()
        .actor_system()
        .spawn::<_, ActorUnprivilegedHandler>(
            crate::actor::ActorUnprivilegedHandlerSpawnSpecification {
                actor: UnprivilegedActor::new(
                    ActorUnprivilegedAddress::new(0),
                ),
                addresses: Vec::default(),
                deadline_in_milliseconds: None,
                marker: PhantomData::<_>,
            }
        );*/

        use zcene_core::future::FutureExt;

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

        let memory_manager = match KernelMemoryManager::new(&logger, boot_info) {
            Ok(memory_manager) => memory_manager,
            Err(error) => {
                logger.writer(|writer| write!(writer, "{:?}", error));
                return Err(error.into());
            }
        };

        let actor_system = ActorSystem::try_new(ActorHandler::new(KernelFutureRuntime::new(
            KernelFutureRuntimeHandler::default(),
        )?))?;

        let timer = KernelTimer::new(
            &memory_manager,
            boot_info
                .rsdp_addr
                .into_option()
                .map(PhysicalMemoryAddress::from),
        );

        //use crate::kernel::actor::actor_system_call_entry_point;
        use x86::msr::wrmsr;
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

        use alloc::boxed::Box;

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

            //DS::set_reg(kernel_data);
            //SS::set_reg(kernel_data);

            wrmsr(IA32_STAR, selector);
            wrmsr(
                IA32_LSTAR,
                crate::actor::actor_system_call_entry_point as u64,
            );
            wrmsr(IA32_FMASK, 0x200);
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

    pub fn actor_system(&self) -> &ActorSystemReference<ActorHandler<KernelFutureRuntimeHandler>> {
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
    config.kernel_stack_size = 8 * 4096;
    config.mappings.physical_memory = Some(Mapping::Dynamic);

    config
};

////////////////////////////////////////////////////////////////////////////////////////////////////

entry_point!(
    Kernel::bootstrap_processor_entry_point,
    config = &BOOTLOADER_CONFIG
);
