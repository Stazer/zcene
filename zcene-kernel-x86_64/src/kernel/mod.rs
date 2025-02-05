mod interrupt_descriptor_table;

use crate::driver::acpi::hpet::HpetRegisters;
pub use interrupt_descriptor_table::*;

////////////////////////////////////////////////////////////////////////////////////////////////////

use crate::architecture::Stack;
use crate::entry_point::{
    double_fault_entry_point, keyboard_interrupt_entry_point, page_fault_entry_point,
    timer_interrupt_entry_point, timer_interrupt_handler, unhandled_interrupt_entry_point,
};
use crate::driver::acpi::hpet::Hpet;
use zcene_kernel::common::volatile::{ReadVolatile, ReadWriteVolatile};
use zcene_kernel::time::Timer;
use alloc::sync::Arc;
use bootloader_api::info::Optional;
use crate::global_allocator::GLOBAL_ALLOCATOR;
use crate::logger::Logger;
use bootloader_api::info::MemoryRegionKind;
use bootloader_api::BootInfo;
use core::cell::SyncUnsafeCell;
use core::fmt::{self, Write};
use core::iter::once;
use core::slice::from_raw_parts_mut;
use pic8259::ChainedPics;
use x2apic::lapic::LocalApic;
use x2apic::lapic::{xapic_base, IpiDestMode, LocalApicBuilder};
use x86::cpuid::CpuId;
use x86::cpuid::TopologyType;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{Mapper, OffsetPageTable, Page, PageTable, PageTableFlags};
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use zcene_kernel::common::linker_value;
use zcene_kernel::memory::address::{
    PhysicalMemoryAddress, PhysicalMemoryAddressPerspective, VirtualMemoryAddress,
    VirtualMemoryAddressPerspective,
};
use zcene_kernel::memory::frame::{FrameManager, FrameManagerAllocationError};
use zcene_kernel::time::{AtomicTimer, TimerInstant};
use crate::driver::acpi::PhysicalMemoryOffsetAcpiHandler;
use acpi::hpet::HpetTable;
use acpi::{AcpiTables, HpetInfo};
use core::time::Duration;
use zcene_kernel::common::As;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct InterruptManager {}

////////////////////////////////////////////////////////////////////////////////////////////////////

const FRAME_SIZE: usize = 4096;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;
const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const TIMER_INTERRUPT_ID: u8 = 0x20;
const SPURIOUS_ID: u8 = 0x20 + 15;
const KEYBOARD_INTERRUPT_ID: u8 = 0x21;

////////////////////////////////////////////////////////////////////////////////////////////////////

use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

pub struct FixedFrameAllocator<T>(T)
where
    T: Iterator<Item = PhysFrame>;

unsafe impl<T> FrameAllocator<Size4KiB> for FixedFrameAllocator<T>
where
    T: Iterator<Item = PhysFrame>,
{
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.0.next()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum InitializeKernelError {
    FrameBufferUnavailable,
    PhysicalMemoryOffsetUnvailable,
    FrameAllocation(FrameManagerAllocationError),
    Fmt(fmt::Error),
    BuildLocalApic(&'static str),
    InitializeMemoryManager(InitializeMemoryManagerError),
}

impl From<fmt::Error> for InitializeKernelError {
    fn from(error: fmt::Error) -> Self {
        Self::Fmt(error)
    }
}

impl From<FrameManagerAllocationError> for InitializeKernelError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl From<InitializeMemoryManagerError> for InitializeKernelError {
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

#[derive(Constructor, Default)]
pub struct LongRunningActor {
    number: usize,
    times: usize,
}

impl<H> Actor<H> for LongRunningActor
where
    H: ActorHandler,
{
    type Message = ();

    fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async move {
            loop {
                crate::common::println!("long running {}", self.number);

                for i in 0..1000000000 {
                    core::hint::black_box(());
                    x86_64::instructions::nop();
                }
            }

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

use zcene_core::actor::{Actor, ActorCreateError, ActorFuture, ActorHandleError, ActorHandler};
use zcene_core::future::runtime::{FutureRuntime, FutureRuntimeActorHandler};

pub type KernelFutureRuntime = FutureRuntime<FutureRuntimeHandler>;

use alloc::alloc::Global;

use crate::future::FutureRuntimeHandler;
use zcene_core::actor::{ActorAddressReference, ActorSystem, ActorSystemReference};

pub type KernelActorHandler = crate::actor::ActorHandler<FutureRuntimeHandler>;
pub type KernelActorSystemReference = ActorSystemReference<KernelActorHandler>;
pub type KernelActorAddress<A> = <KernelActorHandler as ActorHandler>::Address<A>;
pub type KernelActorAddressReference<A> = ActorAddressReference<A, KernelActorHandler>;

use ztd::Method;

#[derive(Debug, Method)]
#[Method(accessors)]
pub struct MemoryManager {
    physical_memory_offset: u64,
    physical_memory_size_in_bytes: u64,
}

#[derive(Debug)]
pub enum InitializeMemoryManagerError {
    UnsupportedMapping,
    FrameAllocation(FrameManagerAllocationError),
}

impl From<FrameManagerAllocationError> for InitializeMemoryManagerError {
    fn from(error: FrameManagerAllocationError) -> Self {
        Self::FrameAllocation(error)
    }
}

impl MemoryManager {
    pub fn new(
        boot_info: &mut BootInfo,
    ) -> Result<Self, InitializeMemoryManagerError> {
        let physical_memory_size_in_bytes = boot_info
            .memory_regions
            .iter()
            .filter(|region| {
                matches!(
                    region.kind,
                    MemoryRegionKind::Usable | MemoryRegionKind::Bootloader
                )
            })
            .map(|region| region.end - region.start)
            .sum::<u64>();

        let physical_memory_offset = boot_info
            .physical_memory_offset
            .into_option()
            .ok_or(InitializeMemoryManagerError::UnsupportedMapping)?;

        let this = Self {
            physical_memory_offset,
            physical_memory_size_in_bytes,
        };

        for entry in this.active_page_table().iter_mut() {
            let mut flags = entry.flags();

            if flags.contains(PageTableFlags::PRESENT) {
                flags.insert(PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE);
                entry.set_flags(flags);
            }
        }

        let mut frame_manager = this.frame_manager();

        for memory_region in boot_info.memory_regions.iter() {
            if matches!(memory_region.kind, MemoryRegionKind::Usable) {
                continue;
            }

            let start = frame_manager
                .translate_memory_address(PhysicalMemoryAddress::from(memory_region.start));
            let end = frame_manager
                .translate_memory_address(PhysicalMemoryAddress::from(memory_region.end));

            for identifier in start..end {
                if frame_manager
                    .translate_frame_identifier(identifier)
                    .as_u64()
                    >= physical_memory_size_in_bytes
                {
                    continue;
                }

                match frame_manager.allocate_frames(once(identifier)) {
                    Ok(()) => {}
                    Err(FrameManagerAllocationError::Allocated(_)) => {}
                    Err(error) => todo!(),
                }
            }
        }

        let frame_count = 10 * 1;
        let heap_frame_identifiers = frame_manager
            .allocate_window(frame_count)?;

        let mut allocator = GLOBAL_ALLOCATOR.inner().lock();

        let memory_address =
            frame_manager
            .translate_frame_identifier(heap_frame_identifiers.last().unwrap());

        unsafe {
            allocator.init(
                (memory_address.as_u64() + this.physical_memory_offset) as *mut u8,
                frame_count * frame_manager.frame_byte_size(),
            );
        }

        Ok(this)
    }

    pub fn physical_memory(&self) -> &'static mut [u8] {
        unsafe {
            from_raw_parts_mut(
                self.physical_memory_offset as *mut u8,
                self.physical_memory_size_in_bytes.try_into().unwrap(),
            )
        }
    }

    pub fn translate_virtual_memory_address(
        &self,
        memory_address: VirtualMemoryAddress,
    ) -> PhysicalMemoryAddress {
        PhysicalMemoryAddress::from(memory_address.as_u64() - self.physical_memory_offset)
    }

    pub fn translate_physical_memory_address(
        &self,
        memory_address: PhysicalMemoryAddress,
    ) -> VirtualMemoryAddress {
        VirtualMemoryAddress::from(memory_address.as_u64() + self.physical_memory_offset)
    }

    pub fn frame_manager(&self) -> FrameManager<'static, PhysicalMemoryAddressPerspective> {
        unsafe { FrameManager::new_initialized(FRAME_SIZE, self.physical_memory()) }
    }

    pub fn active_page_table(&self) -> &'static mut PageTable {
        let pointer = self
            .translate_physical_memory_address(PhysicalMemoryAddress::from(
                Cr3::read().0.start_address().as_u64(),
            ))
            .as_u64();

        unsafe { &mut *(pointer as *mut PageTable) }
    }

    pub fn page_table_mapper(&self) -> OffsetPageTable<'static> {
        unsafe {
            OffsetPageTable::new(
                self.active_page_table(),
                VirtAddr::new(self.physical_memory_offset as u64),
            )
        }
    }

    pub fn allocate_stack(&self) -> Option<Stack<VirtualMemoryAddressPerspective>> {
        let stack_frame_count = 4;
        let stack_size = stack_frame_count * FRAME_SIZE;

        let mut first_address = 0;
        let mut stack_address = 0;
        let mut mapper = self.page_table_mapper();

        for stack_frame_identifier in self.frame_manager().allocate_window(4).unwrap() {
            stack_address = self
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier)
                .as_usize();

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address).try_into().unwrap(),
            ));

            if first_address == 0 {
                first_address = page.start_address().as_u64() + stack_size as u64;
            }

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.try_into().unwrap(),
                        ))
                        .unwrap(),
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut EmptyFrameAllocator,
                    )
                    .expect("Hello World")
                    .flush();
            }
        }

        let first_address = VirtualMemoryAddress::from(first_address);

        Some(Stack::new(first_address, stack_size))
    }
}

pub struct Kernel {
    logger: Logger,
    cores: usize,
    actor_system: KernelActorSystemReference,
    timer_actor: KernelActorAddressReference<TimerActor>,
    memory_manager: MemoryManager,
    timer: KernelTimer<'static>,
}

pub enum KernelTimer<'a> {
    Atomic(AtomicTimer),
    Hpet(Hpet<'a>),
}

impl<'a> KernelTimer<'a> {
    pub fn new(
        memory_manager: &MemoryManager,
        rsdp_address: Option<PhysicalMemoryAddress>,
    ) -> Self {
        rsdp_address
            .map(|rsdp_address| Self::new_hpet(memory_manager, rsdp_address))
            .flatten()
            .unwrap_or_else(Self::new_atomic)
    }

    fn new_hpet(memory_manager: &MemoryManager, rsdp_address: PhysicalMemoryAddress) -> Option<Self> {
        let acpi_tables = unsafe {
            AcpiTables::from_rsdp(
                PhysicalMemoryOffsetAcpiHandler::new(memory_manager.physical_memory_offset().r#as()),
                rsdp_address.as_usize(),
            ).ok()?
        };

        let hpet_address = memory_manager.translate_physical_memory_address(
            PhysicalMemoryAddress::new(HpetInfo::new(&acpi_tables).ok()?.base_address)
        );

        let mut hpet = Hpet::new(
            unsafe {
                hpet_address.cast_mut::<HpetRegisters>().as_mut()?
            }
        );

        hpet.enable();

        Some(Self::Hpet(hpet))
    }

    fn new_atomic() -> Self {
        Self::Atomic(AtomicTimer::new(Duration::from_millis(10)))
    }
}

impl<'a> Timer for KernelTimer<'a> {
    fn now(&self) -> TimerInstant {
        match self {
            Self::Atomic(atomic) => atomic.now(),
            Self::Hpet(hpet) => hpet.now(),
        }
    }

    fn duration_between(&self, start: TimerInstant, end: TimerInstant) -> Duration {
        match self {
            Self::Atomic(atomic) => atomic.duration_between(start, end),
            Self::Hpet(hpet) => hpet.duration_between(start, end),
        }
    }
}

impl Kernel {
    pub fn new(
        boot_info: &'static mut BootInfo,
    ) -> Result<Self, InitializeKernelError> {
        let rsdp_addr = boot_info.rsdp_addr;

        let logger = Logger::new();
        logger.attach_boot_info(
            unsafe {
                (boot_info as *mut BootInfo).as_mut().unwrap()
            }
        );

        let memory_manager = MemoryManager::new(
            unsafe {
                (boot_info as *mut BootInfo).as_mut().unwrap()
            }
        )?;

        let actor_system = ActorSystem::try_new(crate::actor::ActorHandler::new(
                FutureRuntime::new(FutureRuntimeHandler::default()).unwrap(),
                Arc::default(),
            )
        )
            .unwrap();

        let timer = KernelTimer::new(&memory_manager, rsdp_addr.into_option().map(PhysicalMemoryAddress::from));

        let timer_actor = actor_system.spawn(TimerActor::default()).unwrap();

        use zcene_core::actor::ActorAddressExt;
        use zcene_core::future::FutureExt;

        timer_actor
            .send(TimerActorMessage::Subscription(
                actor_system
                    .spawn(ApplicationActor::new(1, 0))
                    .unwrap()
                    .mailbox()
                    .unwrap(),
            ))
            .complete()
            .unwrap();

        timer_actor
            .send(TimerActorMessage::Subscription(
                actor_system
                    .spawn(ApplicationActor::new(2, 0))
                    .unwrap()
                    .mailbox()
                    .unwrap(),
            ))
            .complete()
            .unwrap();

        timer_actor
            .send(TimerActorMessage::Subscription(
                actor_system
                    .spawn(ApplicationActor::new(3, 0))
                    .unwrap()
                    .mailbox()
                    .unwrap(),
            ))
            .complete()
            .unwrap();

        let this = Self {
            logger,
            cores: 1,
            actor_system,
            timer_actor,
            memory_manager,
            timer,
        };

        Ok(this)
    }

    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }

    pub fn get<'a>() -> &'a Kernel {
        unsafe {
            KERNEL.get().as_ref().unwrap().as_ptr().as_ref().unwrap()
        }
    }

    pub fn get_mut<'a>() -> &'a mut Kernel {
        unsafe {
            KERNEL.get().as_mut().unwrap().as_mut_ptr().as_mut().unwrap()
        }
    }

    pub fn timer_actor(&self) -> &KernelActorAddressReference<TimerActor> {
        &self.timer_actor
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    pub fn run(&self) -> ! {
        self.logger().writer(|w| write!(w, "zcene\n",));

        self.initialize_interrupts();

        x86_64::instructions::interrupts::enable();

        self.actor_system().enter().unwrap();

        loop {}
    }

    pub fn actor_system(&self) -> &KernelActorSystemReference {
        &self.actor_system
    }

    pub fn timer(&self) -> &KernelTimer {
        &self.timer
    }

    fn initialize_cores(&mut self) {
        let cpu_id = CpuId::new();

        self.cores = 0;

        for extended_topology_info in cpu_id.get_extended_topology_info().into_iter().flatten() {
            self.cores += match extended_topology_info.level_type() {
                TopologyType::Core => extended_topology_info.processors() as usize,
                _ => 0,
            };
        }
    }

    fn initialize_interrupts(&self) -> Result<(), InitializeKernelError> {
        /*let apic_virtual_address: u64 =
            unsafe { xapic_base() + self.physical_memory_offset as u64 };

        //use x2apic::lapic::{TimerDivide, TimerMode};

        let mut local_apic = LocalApicBuilder::new()
            .timer_vector(TIMER_INTERRUPT_ID.into())
            .error_vector(33)
            .spurious_vector(34)
            .timer_mode(TimerMode::Periodic)
            .timer_divide(TimerDivide::Div16)
            .timer_initial(15_000_000)
            .set_xapic_base(apic_virtual_address)
            .ipi_destination_mode(IpiDestMode::Physical)
            .build()
            .map_err(InitializeKernelError::BuildLocalApic)?;

        unsafe {
            local_apic.enable();
        }*/

        unsafe {
            use x86_64::structures::idt::InterruptDescriptorTable;

            let mut interrupt_descriptor_table = InterruptDescriptorTable::new();
            //let interrupt_descriptor_table = &mut *INTERRUPT_DESCRIPTOR_TABLE.get();

            use crate::entry_point::*;

            interrupt_descriptor_table
                .divide_error
                .set_handler_fn(divide_by_zero_interrupt_entry_point);
            interrupt_descriptor_table
                .debug
                .set_handler_fn(debug_interrupt_entry_point);
            interrupt_descriptor_table
                .non_maskable_interrupt
                .set_handler_fn(non_maskable_interrupt_entry_point);
            interrupt_descriptor_table
                .breakpoint
                .set_handler_fn(breakpoint_interrupt_entry_point);
            interrupt_descriptor_table
                .overflow
                .set_handler_fn(overflow_interrupt_entry_point);
            interrupt_descriptor_table
                .bound_range_exceeded
                .set_handler_fn(bound_range_exceeded_interrupt_entry_point);
            interrupt_descriptor_table
                .invalid_opcode
                .set_handler_fn(invalid_opcode_interrupt_entry_point);
            interrupt_descriptor_table
                .device_not_available
                .set_handler_fn(device_not_available_interrupt_entry_point);
            interrupt_descriptor_table
                .double_fault
                .set_handler_fn(double_fault_entry_point);
            interrupt_descriptor_table
                .invalid_tss
                .set_handler_fn(invalid_tss_entry_point);
            interrupt_descriptor_table
                .segment_not_present
                .set_handler_fn(segment_not_present_entry_point);
            interrupt_descriptor_table
                .stack_segment_fault
                .set_handler_fn(stack_segment_fault_entry_point);
            interrupt_descriptor_table
                .general_protection_fault
                .set_handler_fn(general_protection_fault_entry_point);
            interrupt_descriptor_table
                .page_fault
                .set_handler_fn(page_fault_entry_point);
            interrupt_descriptor_table
                .x87_floating_point
                .set_handler_fn(unhandled_interrupt_entry_point);
            interrupt_descriptor_table
                .alignment_check
                .set_handler_fn(alignment_check_entry_point);
            interrupt_descriptor_table
                .machine_check
                .set_handler_fn(machine_check_interrupt_entry_point);
            interrupt_descriptor_table
                .simd_floating_point
                .set_handler_fn(unhandled_interrupt_entry_point);
            interrupt_descriptor_table
                .virtualization
                .set_handler_fn(virtualization_entry_point);
            interrupt_descriptor_table
                .cp_protection_exception
                .set_handler_fn(cp_protection_entry_point);
            interrupt_descriptor_table
                .hv_injection_exception
                .set_handler_fn(hv_injection_interrupt_entry_point);
            interrupt_descriptor_table
                .vmm_communication_exception
                .set_handler_fn(vmm_entry_point);
            interrupt_descriptor_table
                .security_exception
                .set_handler_fn(security_exception_entry_point);

            for i in EXTERNAL_INTERRUPTS_START..u8::MAX {
                interrupt_descriptor_table[i].set_handler_fn(unhandled_interrupt_entry_point);
            }

            interrupt_descriptor_table[TIMER_INTERRUPT_ID]
                //.set_handler_addr(VirtAddr::new((timer_interrupt_entry_point as usize).try_into().unwrap()));
                .set_handler_addr(VirtAddr::new(
                    (timer_entry_point as usize).try_into().unwrap(),
                ));
            //.set_handler_fn(timer_entry_point);
            interrupt_descriptor_table[KEYBOARD_INTERRUPT_ID]
                .set_handler_fn(keyboard_interrupt_entry_point);

            interrupt_descriptor_table[SPURIOUS_ID].set_handler_fn(spurious_handler);

            interrupt_descriptor_table.load_unsafe();
        }

        let cpu_id = CpuId::new();
        let feature_info = cpu_id.get_feature_info().unwrap();

        use crate::architecture::interrupts::LocalInterruptManager;

        let r#type = LocalInterruptManager::new(self.memory_manager());

        r#type.enable_timer(0x20, core::time::Duration::from_millis(100));

        use x86::apic::ioapic::IoApic;

        /*unsafe {
            let mut apic = IoApic::new((self.physical_memory_offset + 0xFEC00000) as _);
            apic.enable(1, 0);
        }*/

        Ok(())
        //Ok(local_apic)
    }

    fn initialize_smp(&mut self) -> Result<(), InitializeKernelError> {
        let smp_frame_identifier = self
            .memory_manager()
            .frame_manager()
            .unallocated_frame_identifiers()
            .next()
            .unwrap();

        self.memory_manager.frame_manager()
            .allocate_frames(once(smp_frame_identifier))?;

        let smp_memory_address = self
            .memory_manager()
            .frame_manager()
            .translate_frame_identifier(smp_frame_identifier);

        use crate::architecture::smp::SMP_SECTIONS_START;
        use crate::architecture::smp::{SMP_HEADER, SMP_SECTIONS_SIZE};
        use core::slice::{from_raw_parts, from_raw_parts_mut};

        let smp_sections_start = unsafe { linker_value(&SMP_SECTIONS_START) };
        let smp_sections_size = unsafe { linker_value(&SMP_SECTIONS_SIZE) };

        let smp_section_from =
            unsafe { from_raw_parts((smp_sections_start) as *const u8, smp_sections_size) };

        let smp_section_to = unsafe {
            from_raw_parts_mut(
                (self.memory_manager.physical_memory_offset + smp_memory_address.as_u64()) as *mut u8,
                smp_sections_size,
            )
        };

        smp_section_to.copy_from_slice(smp_section_from);

        let mut mapper = self.memory_manager().page_table_mapper();

        let smp_page = Page::<Size4KiB>::containing_address(VirtAddr::new(
            smp_sections_start.try_into().unwrap(),
        ));

        unsafe {
            mapper.unmap(smp_page).expect("hello").1.ignore();
            mapper
                .map_to(
                    smp_page,
                    PhysFrame::from_start_address(PhysAddr::new(0)).unwrap(),
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut EmptyFrameAllocator,
                )
                .expect("dooooO!")
                .flush();
        }

        let stack_frame_count = 4;
        let stack_size = stack_frame_count * FRAME_SIZE;
        let total_stack_size = (self.cores - 1) * stack_size;

        for stack_frame_identifier in self
            .memory_manager()
            .frame_manager()
            .allocate_window((self.cores - 1) * stack_frame_count)?
        {
            let stack_address = self
                .memory_manager()
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier);

            use x86_64::instructions::tables::sgdt;

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address.as_usize()).try_into().unwrap(),
            ));

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.as_usize().try_into().unwrap(),
                        ))
                        .unwrap(),
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut EmptyFrameAllocator,
                    )
                    .expect("Hello World")
                    .flush();
            }

            unsafe {
                if SMP_HEADER.stack_start == 0 {
                    SMP_HEADER.stack_start = page.start_address().as_u64() + stack_size as u64;
                    SMP_HEADER.stack_size = stack_size as _;
                    SMP_HEADER.page_table_start =
                        Cr3::read().0.start_address().as_u64().try_into().expect("");
                    SMP_HEADER.gdt64_pointer = sgdt();
                }
            }
        }

        Ok(())
    }

    fn boot_application_processors(&mut self, mut local_apic: LocalApic) {
        unsafe {
            local_apic.send_init_ipi_all();

            for i in 0..100000000 {
                core::hint::black_box(());
                x86_64::instructions::nop();
            }

            local_apic.send_sipi_all(
                (crate::architecture::smp::smp_real_mode_entry as u64)
                    .try_into()
                    .unwrap(),
            );
        }
    }

}

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::mem::MaybeUninit;

pub static KERNEL: SyncUnsafeCell<MaybeUninit<Kernel>> = SyncUnsafeCell::new(MaybeUninit::zeroed());
