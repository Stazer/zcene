mod interrupt_descriptor_table;

pub use interrupt_descriptor_table::*;

////////////////////////////////////////////////////////////////////////////////////////////////////

use crate::entry_point::{
    double_fault_entry_point, keyboard_interrupt_entry_point, page_fault_entry_point,
    timer_interrupt_entry_point, unhandled_interrupt_entry_point,
};
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
use zcene_kernel::common::memory::{MemoryAddress, PhysicalMemoryAddressPerspective};
use zcene_kernel::memory::frame::{FrameManager, FrameManagerAllocationError};
use zcene_core::actor::ActorAddressExt;

////////////////////////////////////////////////////////////////////////////////////////////////////

const FRAME_SIZE: usize = 4096;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;
const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const TIMER_INTERRUPT_ID: u8 = 0x20;
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

pub struct PrintCpuIdActor;

impl PrintCpuIdActor {
    async fn print(&self) {
        let cpu_id = CpuId::new();
        let feature_info = cpu_id.get_feature_info().unwrap();

        Kernel::get().logger().writer(|w| {
            write!(
                w,
                "Hello World from {}!!!!!!!\n",
                feature_info.initial_local_apic_id()
            )
        });
    }
}

impl<H> Actor<H> for PrintCpuIdActor
where
    H: ActorHandler,
{
    type Message = ();

    fn create(
        &mut self,
        _context: H::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        async {
            self.print().await;

            Ok(())
        }
    }

    fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async {
            self.print().await;

            Ok(())
        }
    }
}

#[derive(Default)]
pub struct ApplicationActor {}

impl<H> Actor<H> for ApplicationActor
where
    H: ActorHandler,
{
    type Message = ();

    fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async {
            Kernel::get()
                .logger()
                .writer(|w| write!(w, "Hello World\n",));

            Ok(())
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

use alloc::vec::Vec;
use zcene_core::actor::{ActorMailbox, ActorMessageSender};
use ztd::Constructor;
use zcene_core::actor::ActorContextMessageProvider;

#[derive(Default, Constructor)]
pub struct TimerActor {
    subscriptions: Vec<ActorMailbox<(), KernelActorHandler>>,
    total_ticks: usize,
}

#[derive(Clone)]
pub enum TimerActorMessage {
    Tick,
    Subscription(ActorMailbox<(), KernelActorHandler>),
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
                        subscription.send(()).await;
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
use zcene_core::future::runtime::{
    FutureRuntime, FutureRuntimeActorHandler, FutureRuntimeConcurrentQueue,
    FutureRuntimeContinueWaker, FutureRuntimeNoOperationYielder,
};

pub type KernelFutureRuntime = FutureRuntime<FutureRuntimeHandler>;

use alloc::alloc::Global;

use zcene_core::actor::{ActorAddressReference, ActorSystem, ActorSystemReference};
use crate::common::future::FutureRuntimeHandler;

pub type KernelActorHandler = FutureRuntimeActorHandler<FutureRuntimeHandler>;
pub type KernelActorSystemReference = ActorSystemReference<KernelActorHandler>;
pub type KernelActorAddress<A> = <KernelActorHandler as ActorHandler>::Address<A>;
pub type KernelActorAddressReference<A> = ActorAddressReference<A, KernelActorHandler>;

pub struct Kernel {
    logger: Logger,
    cores: usize,
    physical_memory_offset: usize,
    physical_memory_size_in_bytes: usize,
    actor_system: Option<KernelActorSystemReference>,
    timer_actor: Option<KernelActorAddressReference<TimerActor>>,
}

impl Kernel {
    pub fn get<'a>() -> &'a Kernel {
        unsafe { &*KERNEL.get() }
    }

    pub fn get_mut<'a>() -> &'a mut Kernel {
        unsafe { &mut *KERNEL.get() }
    }

    pub fn initialize<'a>(
        &mut self,
        boot_info: &'a mut BootInfo,
    ) -> Result<(), InitializeKernelError>
    where
        'a: 'static,
    {
        self.initialize_physical_memory(boot_info)?;
        self.initialize_frame_manager(boot_info)?;

        self.logger.attach_boot_info(boot_info);

        self.initialize_cores();
        let local_apic = self.initialize_interrupts()?;
        self.initialize_smp()?;

        self.initialize_heap()?;

        self.actor_system = Some(
            ActorSystem::try_new(FutureRuntimeActorHandler::new(
                FutureRuntime::new(FutureRuntimeHandler::default()).unwrap(),
            ))
            .unwrap(),
        );

        let timer_actor = self.actor_system().spawn(TimerActor::default()).unwrap();

        self.timer_actor = Some(timer_actor.clone());

        use zcene_core::future::FutureExt;

        /*timer_actor
            .send(TimerActorMessage::Subscribe(
            ))
            .complete();*/

        self.boot_application_processors(local_apic);

        x86_64::instructions::interrupts::enable();

        self.logger().writer(|w| write!(w, "zcene\n",))?;

        Ok(())
    }

    pub fn timer_actor(&self) -> &KernelActorAddressReference<TimerActor> {
        self.timer_actor.as_ref().unwrap()
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    pub fn run(&self) -> ! {
        self.actor_system.as_ref().unwrap().enter();

        loop {}
    }

    pub fn physical_memory(&self) -> &'static mut [u8] {
        unsafe {
            from_raw_parts_mut(
                self.physical_memory_offset as *mut u8,
                self.physical_memory_size_in_bytes,
            )
        }
    }

    pub fn active_page_table(&self) -> &'static mut PageTable {
        let (level_4_table_frame, _) = Cr3::read();

        let phys = level_4_table_frame.start_address().as_u64() as usize;
        let virt = (self.physical_memory_offset + phys) as usize;
        let page_table_ptr: *mut PageTable = virt as _;

        unsafe { &mut *page_table_ptr }
    }

    pub fn page_table_mapper(&self) -> OffsetPageTable<'static> {
        unsafe {
            OffsetPageTable::new(
                self.active_page_table(),
                VirtAddr::new(self.physical_memory_offset as u64),
            )
        }
    }

    pub fn frame_manager(&self) -> FrameManager<'static, PhysicalMemoryAddressPerspective> {
        unsafe { FrameManager::new_initialized(FRAME_SIZE, self.physical_memory()) }
    }

    pub fn actor_system(&self) -> &KernelActorSystemReference {
        self.actor_system.as_ref().unwrap()
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

    fn initialize_physical_memory(
        &mut self,
        boot_info: &mut BootInfo,
    ) -> Result<(), InitializeKernelError> {
        self.physical_memory_size_in_bytes = boot_info
            .memory_regions
            .iter()
            .filter(|region| {
                matches!(
                    region.kind,
                    MemoryRegionKind::Usable | MemoryRegionKind::Bootloader
                )
            })
            .map(|region| region.end - region.start)
            .sum::<u64>() as usize;

        self.physical_memory_offset = boot_info
            .physical_memory_offset
            .into_option()
            .ok_or(InitializeKernelError::PhysicalMemoryOffsetUnvailable)?
            as usize;

        for entry in self.active_page_table().iter_mut() {
            let mut flags = entry.flags();

            if flags.contains(PageTableFlags::PRESENT) {
                flags.insert(PageTableFlags::WRITABLE);
                entry.set_flags(flags);
            }
        }

        Ok(())
    }

    fn initialize_frame_manager(
        &mut self,
        boot_info: &mut BootInfo,
    ) -> Result<(), InitializeKernelError> {
        let mut frame_manager = FrameManager::<'_, PhysicalMemoryAddressPerspective>::new(
            FRAME_SIZE,
            self.physical_memory(),
        );

        let physical_memory_size_in_bytes = self.physical_memory_size_in_bytes;

        for memory_region in boot_info.memory_regions.iter() {
            if matches!(memory_region.kind, MemoryRegionKind::Usable) {
                continue;
            }

            let start =
                frame_manager.translate_memory_address(MemoryAddress::new(memory_region.start));
            let end = frame_manager.translate_memory_address(MemoryAddress::new(memory_region.end));

            for identifier in start..end {
                if frame_manager
                    .translate_frame_identifier(identifier)
                    .as_usize()
                    >= physical_memory_size_in_bytes
                {
                    continue;
                }

                match frame_manager.allocate_frames(once(identifier)) {
                    Ok(()) => {}
                    Err(FrameManagerAllocationError::Allocated(_)) => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }

        Ok(())
    }

    fn initialize_interrupts(&self) -> Result<LocalApic, InitializeKernelError> {
        unsafe {
            let mut pic = ChainedPics::new(PARENT_PIC_OFFSET, CHILD_PIC_OFFSET);
            pic.disable();
        }

        let apic_virtual_address: u64 =
            unsafe { xapic_base() + self.physical_memory_offset as u64 };

        use x2apic::lapic::{TimerDivide, TimerMode};

        let mut local_apic = LocalApicBuilder::new()
            .timer_vector(TIMER_INTERRUPT_ID.into())
            .error_vector(33)
            .spurious_vector(34)
            .timer_mode(TimerMode::Periodic)
            .timer_divide(TimerDivide::Div16)
            .timer_initial(1_250_000)
            .set_xapic_base(apic_virtual_address)
            .ipi_destination_mode(IpiDestMode::Physical)
            .build()
            .map_err(InitializeKernelError::BuildLocalApic)?;

        unsafe {
            local_apic.enable();
        }

        unsafe {
            let interrupt_descriptor_table = &mut *INTERRUPT_DESCRIPTOR_TABLE.get();

            for i in EXTERNAL_INTERRUPTS_START..u8::MAX {
                interrupt_descriptor_table[i].set_handler_fn(unhandled_interrupt_entry_point);
            }

            interrupt_descriptor_table
                .double_fault
                .set_handler_fn(double_fault_entry_point);
            interrupt_descriptor_table
                .page_fault
                .set_handler_fn(page_fault_entry_point);

            interrupt_descriptor_table[TIMER_INTERRUPT_ID]
                .set_handler_fn(timer_interrupt_entry_point);
            interrupt_descriptor_table[KEYBOARD_INTERRUPT_ID]
                .set_handler_fn(keyboard_interrupt_entry_point);

            interrupt_descriptor_table.load();
        }

        use x86::apic::ioapic::IoApic;

        unsafe {
            let mut apic = IoApic::new((self.physical_memory_offset + 0xFEC00000) as _);
            apic.enable(1, 0);
        }

        Ok(local_apic)
    }

    fn initialize_heap(&mut self) -> Result<(), InitializeKernelError> {
        let frame_count = 1024;
        let heap_frame_identifiers = self.frame_manager().allocate_window(frame_count)?;

        let mut allocator = GLOBAL_ALLOCATOR.lock();

        let memory_address = self
            .frame_manager()
            .translate_frame_identifier(heap_frame_identifiers.last().unwrap());

        unsafe {
            allocator.init(
                (memory_address.as_usize() + self.physical_memory_offset) as *mut u8,
                1024 * self.frame_manager().frame_byte_size(),
            );
        }

        Ok(())
    }

    fn initialize_smp(&mut self) -> Result<(), InitializeKernelError> {
        let smp_frame_identifier = self
            .frame_manager()
            .unallocated_frame_identifiers()
            .next()
            .unwrap();

        self.frame_manager()
            .allocate_frames(once(smp_frame_identifier))?;
        let smp_memory_address = self
            .frame_manager()
            .translate_frame_identifier(smp_frame_identifier);

        use crate::smp::SMP_SECTIONS_START;
        use crate::smp::{SMP_HEADER, SMP_SECTIONS_SIZE};
        use core::slice::{from_raw_parts, from_raw_parts_mut};

        let smp_sections_start = unsafe { linker_value(&SMP_SECTIONS_START) };
        let smp_sections_size = unsafe { linker_value(&SMP_SECTIONS_SIZE) };

        let smp_section_from =
            unsafe { from_raw_parts((smp_sections_start) as *const u8, smp_sections_size) };

        let smp_section_to = unsafe {
            from_raw_parts_mut(
                (self.physical_memory_offset + smp_memory_address.as_usize()) as *mut u8,
                smp_sections_size,
            )
        };

        smp_section_to.copy_from_slice(smp_section_from);

        let mut mapper = self.page_table_mapper();

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
            .frame_manager()
            .allocate_window((self.cores - 1) * stack_frame_count)?
        {
            let stack_address = self
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

            local_apic.send_sipi_all((crate::smp::smp_real_mode_entry as u64).try_into().unwrap());
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

static KERNEL: SyncUnsafeCell<Kernel> = SyncUnsafeCell::new(Kernel {
    logger: Logger::new(),
    cores: 1,
    physical_memory_offset: 0,
    physical_memory_size_in_bytes: 0,
    actor_system: None,
    timer_actor: None,
});
