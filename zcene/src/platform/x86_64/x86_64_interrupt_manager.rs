use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::mem::MaybeUninit;
use core::time::Duration;
use crate::common::memory::allocator::ErasedAllocator;
use crate::common::synchronization::Mutex;
use crate::common::time::Timer;
use crate::driver::xapic::XApic;
use crate::driver::xapic::XApicRegisters;
use crate::platform::x86_64::ExecutionUnitIdentifier;
use crate::platform::x86_64::X86_64MemoryManager;
use crate::utility::println;
use pic8259::ChainedPics;
use x86::cpuid::CpuId;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

////////////////////////////////////////////////////////////////////////////////////////////////////

const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START as u8;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;

pub enum X86_64X86_64LocalInterruptManagerType {
    PIC,
    XAPIC {
        base: *mut u32,
        ticks_per_millisecond: u32,
    },
    X2APIC,
}

pub type InterruptEntryPoint = extern "x86-interrupt" fn(InterruptStackFrame);

pub type InterruptVectorIdentifier = u8;

pub type InterruptHandler = Arc<dyn Fn() + Send + Sync, ErasedAllocator>;

unsafe impl Sync for X86_64X86_64LocalInterruptManagerType {}
unsafe impl Send for X86_64X86_64LocalInterruptManagerType {}

impl X86_64X86_64LocalInterruptManagerType {}

pub struct X86_64LocalInterruptManager {
    r#type: X86_64X86_64LocalInterruptManagerType,
    descriptor_table: Box<InterruptDescriptorTable, ErasedAllocator>,
    free_vectors: BTreeSet<InterruptVectorIdentifier, ErasedAllocator>,
    handlers: Box<[Vec<InterruptHandler, ErasedAllocator>; 256], ErasedAllocator>,
    oneshot_vector: Option<InterruptVectorIdentifier>,
}

pub type InterruptIdentifier = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, core::marker::ConstParamTy)]
pub enum InterruptName {
    DivideError,
    Debug,
    NonMaskableInterrupt,
    Breakpoint,
    Overflow,
    BoundRangeExceeded,
    InvalidOpcode,
    DeviceNotAvailable,
    DoubleFault,
    InvalidTss,
    SegmentNotPresent,
    StackSegmentFault,
    GeneralProtectionFault,
    PageFault,
    X87FloatingPoint,
    AlignmentCheck,
    MachineCheck,
    SimdFloatingPoint,
    Virtualization,
    CpProtectionException,
    HvInjectionException,
    VmmCommunicationException,
    SecurityException,
    Interrupt,
}

impl InterruptName {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DivideError => "divide error",
            Self::Debug => "debug",
            Self::NonMaskableInterrupt => "non maskable interrupt",
            Self::Breakpoint => "breakpoint",
            Self::Overflow => "overflow",
            Self::BoundRangeExceeded => "bound range",
            Self::InvalidOpcode => "invalid opcode",
            Self::DeviceNotAvailable => "device not available",
            Self::DoubleFault => "double fault",
            Self::InvalidTss => "invalid tss",
            Self::SegmentNotPresent => "segment not present",
            Self::StackSegmentFault => "stack segment fault",
            Self::GeneralProtectionFault => "general protection fault",
            Self::PageFault => "page fault",
            Self::X87FloatingPoint => "x87 floating point",
            Self::AlignmentCheck => "alignment check",
            Self::MachineCheck => "machine check",
            Self::SimdFloatingPoint => "simd floating point",
            Self::Virtualization => "virtualization",
            Self::CpProtectionException => "cp protection exception",
            Self::HvInjectionException => "hv injection exception",
            Self::VmmCommunicationException => "vmm communication exception",
            Self::SecurityException => "security exception",
            Self::Interrupt => "interrupt",
        }
    }
}

impl core::fmt::Display for InterruptName {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point<const N: InterruptName>(
    stack_frame: InterruptStackFrame,
) {
    println!("unhandled interrupt {}\n{:?}", N, stack_frame);
}

extern "x86-interrupt" fn external_interrupt_entry_point<const VECTOR: u8>(
    _stack_frame: InterruptStackFrame,
) {
    crate::actor::ActorRootEnvironment::get()
        .platform()
        .interrupt_manager()
        .dispatch(VECTOR);
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point_loop<const N: InterruptName>(
    stack_frame: InterruptStackFrame,
) -> ! {
    println!("unhandled interrupt {}\n{:?}", N, stack_frame);
    loop {}
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point_with_error_code<
    const N: InterruptName,
    E,
>(
    stack_frame: InterruptStackFrame,
    error_code: E,
) where
    E: Debug,
{
    println!(
        "unhandled interrupt {} (0x{:X?})\n{:X?}",
        N, error_code, stack_frame
    );

    loop {}
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point_with_error_code_loop<
    const N: InterruptName,
    E,
>(
    stack_frame: InterruptStackFrame,
    error_code: E,
) -> !
where
    E: Debug,
{
    println!(
        "unhandled interrupt {} {:?}\n{:?}",
        N, error_code, stack_frame
    );
    loop {}
}

impl X86_64LocalInterruptManager {
    pub fn new(timer: &impl Timer, memory_manager: &X86_64MemoryManager) -> Self {
        let mut table = Box::new_in(
            InterruptDescriptorTable::new(),
            memory_manager.heap_allocator(),
        );

        table.divide_error.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::DivideError }>,
        );
        table
            .debug
            .set_handler_fn(unhandled_interrupt_entry_point::<{ InterruptName::Debug }>);
        table.non_maskable_interrupt.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::NonMaskableInterrupt }>,
        );
        table.breakpoint.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::Breakpoint }>,
        );
        table
            .overflow
            .set_handler_fn(unhandled_interrupt_entry_point::<{ InterruptName::Overflow }>);
        table.bound_range_exceeded.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::BoundRangeExceeded }>,
        );
        table.invalid_opcode.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::InvalidOpcode }>,
        );
        table.device_not_available.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::DeviceNotAvailable }>,
        );
        table.double_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code_loop::<
                { InterruptName::DoubleFault },
                _,
            >,
        );
        table.invalid_tss.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<{ InterruptName::InvalidTss }, _>,
        );
        table.segment_not_present.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::SegmentNotPresent },
                _,
            >,
        );
        table.stack_segment_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::StackSegmentFault },
                _,
            >,
        );
        table.general_protection_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::GeneralProtectionFault },
                _,
            >,
        );
        table.page_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<{ InterruptName::PageFault }, _>,
        );
        table.x87_floating_point.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::X87FloatingPoint }>,
        );
        table.alignment_check.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::AlignmentCheck },
                _,
            >,
        );
        table.machine_check.set_handler_fn(
            unhandled_interrupt_entry_point_loop::<{ InterruptName::MachineCheck }>,
        );
        table.simd_floating_point.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::SimdFloatingPoint }>,
        );
        table.virtualization.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::Virtualization }>,
        );
        table.cp_protection_exception.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::CpProtectionException },
                _,
            >,
        );
        table.hv_injection_exception.set_handler_fn(
            unhandled_interrupt_entry_point::<{ InterruptName::HvInjectionException }>,
        );
        table.vmm_communication_exception.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::VmmCommunicationException },
                _,
            >,
        );
        table.security_exception.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<
                { InterruptName::SecurityException },
                _,
            >,
        );

        let mut free_vectors = BTreeSet::new_in(memory_manager.heap_allocator());

        let mut handlers: Box<
            MaybeUninit<[Vec<InterruptHandler, ErasedAllocator>; 256]>,
            ErasedAllocator,
        > = Box::new_uninit_in(memory_manager.heap_allocator());

        let handlers_base = handlers.as_mut_ptr() as *mut Vec<InterruptHandler, ErasedAllocator>;

        for index in 0..256 {
            unsafe {
                handlers_base
                    .add(index)
                    .write(Vec::new_in(memory_manager.heap_allocator()));
            }
        }

        let handlers = unsafe { handlers.assume_init() };

        use x86_64::structures::idt::Entry;

        for vector in EXTERNAL_INTERRUPTS_START..255u8 {
            let mut entry: Entry<_> = table[vector];
            entry.set_handler_fn(unhandled_interrupt_entry_point::<{ InterruptName::Interrupt }>);

            free_vectors.insert(vector.into());
        }

        // Test wiring for the registry-backed dispatch path (`external_interrupt_entry_point`):
        // just one vector for now, to prove the pattern before generating the full 32..255 pool.
        table[EXTERNAL_INTERRUPTS_START]
            .set_handler_fn(external_interrupt_entry_point::<{ EXTERNAL_INTERRUPTS_START }>);

        unsafe {
            table.load_unsafe();
        }

        let cpu_id = CpuId::new();
        let feature_info = cpu_id.get_feature_info();

        if feature_info
            .as_ref()
            .map(|f| f.has_apic())
            .unwrap_or_default()
        {
            unsafe {
                let mut pic = ChainedPics::new(PARENT_PIC_OFFSET, CHILD_PIC_OFFSET);
                pic.disable();
            }

            let base = memory_manager
                .translate_physical_memory_address(XApic::base_address())
                .cast_mut::<u32>();

            let mut xapic = unsafe { XApic::new((base as *mut XApicRegisters).as_mut().unwrap()) };

            let ticks_per_millisecond = xapic.calibrate(timer, Duration::from_millis(1));

            return Self {
                r#type: X86_64X86_64LocalInterruptManagerType::XAPIC {
                    base,
                    ticks_per_millisecond,
                },
                free_vectors,
                descriptor_table: table,
                handlers,
                oneshot_vector: None,
            };
        }

        if feature_info
            .as_ref()
            .map(|f| f.has_x2apic())
            .unwrap_or_default()
        {
            unsafe {
                let mut pic = ChainedPics::new(PARENT_PIC_OFFSET, CHILD_PIC_OFFSET);
                pic.disable();
            }

            todo!()
        }

        todo!()
    }

    pub fn allocate(
        &mut self,
        entry_point: InterruptEntryPoint,
    ) -> Option<InterruptVectorIdentifier> {
        let free_vector = self.free_vectors.pop_first()?;

        unsafe {
            let _entry = self.descriptor_table[free_vector]
                .set_handler_fn(entry_point)
                .set_stack_index(0);
            //.set_privilege_level(x86_64::PrivilegeLevel::Ring3) // Erlaubt User Mode
            //.set_code_selector(SegmentSelector::new(1, PrivilegeLevel::Ring0))
            //.set_present(true)
            //.set_stack_index(0)
            //.set_privilege_level(PrivilegeLevel::Ring3)
            //.set_code_selector(SegmentSelector::new(4, PrivilegeLevel::Ring3));
        }
        //entry.set_privilege_level(PrivilegeLevel::Ring3);
        //entry.set_code_selector(SegmentSelector::new(0x08, PrivilegeLevel::Ring0));

        unsafe {
            self.descriptor_table.load_unsafe();
        }

        Some(free_vector)
    }

    /// Reserves a free vector for `handler` without touching the IDT (dispatch from the
    /// hardware vector to this registry is not wired up yet, see the `handlers` field).
    pub fn register_handler(&mut self, handler: InterruptHandler) -> Option<InterruptVectorIdentifier> {
        let free_vector = self.free_vectors.pop_first()?;

        self.handlers[free_vector as usize].push(handler);

        Some(free_vector)
    }

    pub fn enable_timer(&mut self, entry_point: InterruptEntryPoint, _duration: Duration) {
        match self.r#type {
            X86_64X86_64LocalInterruptManagerType::XAPIC {
                base,
                ticks_per_millisecond,
            } => unsafe {
                let vector = self.allocate(entry_point).unwrap();

                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());
                xapic.enable_timer(vector, ticks_per_millisecond);
            },
            _ => todo!(),
        }
    }

    pub fn enable_oneshot(&mut self, entry_point: InterruptEntryPoint, duration: Duration) {
        match self.r#type {
            X86_64X86_64LocalInterruptManagerType::XAPIC {
                base,
                ticks_per_millisecond,
            } => unsafe {
                let vector = self.allocate(entry_point).unwrap();
                self.oneshot_vector = Some(vector);

                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());

                xapic.enable_oneshot(
                    vector,
                    (duration.as_millis() as u32) * ticks_per_millisecond,
                );
            },
            _ => todo!(),
        }
    }

    pub fn reset_oneshot(&self, duration: Duration) {
        match self.r#type {
            X86_64X86_64LocalInterruptManagerType::XAPIC {
                base,
                ticks_per_millisecond,
            } => unsafe {
                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());

                xapic.reset_oneshot((duration.as_millis() as u32) * ticks_per_millisecond);
            },
            _ => todo!(),
        }
    }

    pub fn notify_end_of_interrupt(&self) {
        match self.r#type {
            X86_64X86_64LocalInterruptManagerType::XAPIC { base, .. } => unsafe {
                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());
                xapic.signal_end_of_interrupt();
            },
            _ => todo!(),
        }
    }

    /// Loads this interrupt descriptor table into the calling core's IDTR. The table is shared
    /// across cores (no per-core customization yet), so every core that wants to survive a CPU
    /// exception without triple-faulting must call this once after it boots.
    pub fn load_descriptor_table(&self) {
        unsafe {
            self.descriptor_table.load_unsafe();
        }
    }

    /// Configures the calling core's local APIC like the bootstrap core's: software-enabled and
    /// with the one-shot timer on the same vector (stopped). The xAPIC MMIO window is per core
    /// at the same address, so the bootstrap core's base works here too.
    pub fn initialize_current_local_apic(&self) {
        if let X86_64X86_64LocalInterruptManagerType::XAPIC { base, .. } = self.r#type {
            unsafe {
                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());
                // An application processor comes out of INIT with its local APIC disabled.
                xapic.set_spurious_interrupt_vector(0xFF);

                if let Some(vector) = self.oneshot_vector {
                    xapic.enable_oneshot(vector, 0);
                }
            }
        }
    }

    pub fn xapic_base(&self) -> Option<*mut u32> {
        match self.r#type {
            X86_64X86_64LocalInterruptManagerType::XAPIC { base, .. } => Some(base),
            _ => None,
        }
    }
}

pub struct X86_64InterruptManager {
    local_interrupt_managers:
        Mutex<BTreeMap<ExecutionUnitIdentifier, X86_64LocalInterruptManager, ErasedAllocator>>,
}

impl X86_64InterruptManager {
    pub fn new(allocator: ErasedAllocator) -> Self {
        Self {
            local_interrupt_managers: Mutex::new(BTreeMap::new_in(allocator)),
        }
    }

    pub fn bootstrap_local_interrupt_manager(
        &mut self,
        local_interrupt_manager: X86_64LocalInterruptManager,
    ) {
        let mut local_interrupt_managers = self.local_interrupt_managers.lock();
        local_interrupt_managers.insert(0, local_interrupt_manager);
    }

    pub fn notify_local_end_of_interrupt(&self) {
        self.local_interrupt_managers
            .lock()
            .get(&0)
            .unwrap()
            .notify_end_of_interrupt();
    }

    pub fn dispatch(&self, vector: InterruptVectorIdentifier) {
        // Cloning `Arc`s and dropping the lock immediately, instead of holding it across the
        // handler calls below: those can take an unbounded amount of time (actor mailbox sends,
        // allocation, ...), and this lock is shared across *all* cores' local managers, so
        // holding it that long would stall every other core's interrupt handling too.
        let handlers = self
            .local_interrupt_managers
            .lock()
            .get(&0)
            .unwrap()
            .handlers[vector as usize]
            .clone();

        for handler in handlers.iter() {
            handler();
        }

        self.notify_local_end_of_interrupt();
    }

    pub fn reset_oneshot(&self, duration: Duration) {
        self.local_interrupt_managers
            .lock()
            .get(&0)
            .unwrap()
            .reset_oneshot(duration);
    }

    /// Loads the bootstrap core's (shared) interrupt descriptor table into the calling core's
    /// IDTR. Intended to be called by an application processor right after it boots.
    pub fn load_bootstrap_descriptor_table(&self) {
        self.local_interrupt_managers
            .lock()
            .get(&0)
            .unwrap()
            .load_descriptor_table();
    }

    pub fn initialize_application_processor_local_apic(&self) {
        self.local_interrupt_managers
            .lock()
            .get(&0)
            .unwrap()
            .initialize_current_local_apic();
    }

    pub fn bootstrap_xapic_base(&self) -> Option<*mut u32> {
        self.local_interrupt_managers.lock().get(&0).unwrap().xapic_base()
    }
}
