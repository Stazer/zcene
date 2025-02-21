use crate::architecture::ExecutionUnitIdentifier;
use crate::driver::xapic::XApic;
use crate::driver::xapic::XApicRegisters;
use crate::kernel::logger::println;
use crate::kernel::memory::KernelMemoryManager;
use crate::kernel::KernelTimer;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use core::fmt::Debug;
use core::time::Duration;
use pic8259::ChainedPics;
use x86::cpuid::CpuId;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;
use zcene_bare::synchronization::Mutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START as u8;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;

pub enum LocalInterruptManagerType {
    PIC,
    XAPIC {
        base: *mut u32,
        ticks_per_millisecond: u32,
    },
    X2APIC,
}

pub type InterruptEntryPoint = extern "x86-interrupt" fn(InterruptStackFrame);

pub type InterruptVectorIdentifier = u8;

unsafe impl Sync for LocalInterruptManagerType {}
unsafe impl Send for LocalInterruptManagerType {}

impl LocalInterruptManagerType {}

pub struct LocalInterruptManager {
    r#type: LocalInterruptManagerType,
    descriptor_table: Box<InterruptDescriptorTable>,
    free_vectors: BTreeSet<InterruptVectorIdentifier>,
}

pub type InterruptIdentifier = u8;

#[inline(always)]
extern "x86-interrupt" fn unhandled_interrupt_loop() -> ! {
    loop {}
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point<const N: &'static str>(
    stack_frame: InterruptStackFrame,
) {
    println!("unhandled interrupt {}\n{:?}", N, stack_frame);
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point_loop<const N: &'static str>(
    stack_frame: InterruptStackFrame,
) -> ! {
    unhandled_interrupt_entry_point::<N>(stack_frame);
    unhandled_interrupt_loop()
}

extern "x86-interrupt" fn unhandled_interrupt_entry_point_with_error_code<
    const N: &'static str,
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
    const N: &'static str,
    E,
>(
    stack_frame: InterruptStackFrame,
    error_code: E,
) -> !
where
    E: Debug,
{
    unhandled_interrupt_entry_point_with_error_code::<N, E>(stack_frame, error_code);
    unhandled_interrupt_loop()
}

impl LocalInterruptManager {
    pub fn new(timer: &KernelTimer, memory_manager: &KernelMemoryManager) -> Self {
        let mut table = Box::new(InterruptDescriptorTable::new());

        table
            .divide_error
            .set_handler_fn(unhandled_interrupt_entry_point::<"divide error">);
        table
            .debug
            .set_handler_fn(unhandled_interrupt_entry_point::<"debug">);
        table
            .non_maskable_interrupt
            .set_handler_fn(unhandled_interrupt_entry_point::<"non maskable interrupt">);
        table
            .breakpoint
            .set_handler_fn(unhandled_interrupt_entry_point::<"breakpoint">);
        table
            .overflow
            .set_handler_fn(unhandled_interrupt_entry_point::<"overflow">);
        table
            .bound_range_exceeded
            .set_handler_fn(unhandled_interrupt_entry_point::<"bound range">);
        table
            .invalid_opcode
            .set_handler_fn(unhandled_interrupt_entry_point::<"invalid opcode">);
        table
            .device_not_available
            .set_handler_fn(unhandled_interrupt_entry_point::<"device not available">);
        table.double_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code_loop::<"double fault", _>,
        );
        table
            .invalid_tss
            .set_handler_fn(unhandled_interrupt_entry_point_with_error_code::<"invalid tss", _>);
        table.segment_not_present.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"segment not present", _>,
        );
        table.stack_segment_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"stack segment fault", _>,
        );
        table.general_protection_fault.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"general protection fault", _>,
        );
        table
            .page_fault
            .set_handler_fn(unhandled_interrupt_entry_point_with_error_code::<"page fault", _>);
        table
            .x87_floating_point
            .set_handler_fn(unhandled_interrupt_entry_point::<"x87 floating point">);
        table.alignment_check.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"alignment check", _>,
        );
        table
            .machine_check
            .set_handler_fn(unhandled_interrupt_entry_point_loop::<"machine check">);
        table
            .simd_floating_point
            .set_handler_fn(unhandled_interrupt_entry_point::<"simd floating point">);
        table
            .virtualization
            .set_handler_fn(unhandled_interrupt_entry_point::<"virtualization">);
        table.cp_protection_exception.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"cp protection exception", _>,
        );
        table
            .hv_injection_exception
            .set_handler_fn(unhandled_interrupt_entry_point::<"hv injection exception">);
        table.vmm_communication_exception.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"vmm communication exception", _>,
        );
        table.security_exception.set_handler_fn(
            unhandled_interrupt_entry_point_with_error_code::<"security exception", _>,
        );

        let mut free_vectors = BTreeSet::default();

        use x86_64::structures::idt::Entry;

        for vector in EXTERNAL_INTERRUPTS_START..255u8 {
            let mut entry: Entry<_> = table[vector];
            entry.set_handler_fn(unhandled_interrupt_entry_point::<"interrupt">);

            free_vectors.insert(vector.into());
        }

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
                r#type: LocalInterruptManagerType::XAPIC {
                    base,
                    ticks_per_millisecond,
                },
                free_vectors,
                descriptor_table: table,
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
            let entry = self.descriptor_table[free_vector]
            .set_handler_fn(entry_point)
            .set_stack_index(0)
            //.set_privilege_level(x86_64::PrivilegeLevel::Ring3) // Erlaubt User Mode
            //.set_code_selector(SegmentSelector::new(1, PrivilegeLevel::Ring0))
            //.set_present(true)
            //.set_stack_index(0)
                ;
            //.set_privilege_level(PrivilegeLevel::Ring3)
            //.set_code_selector(SegmentSelector::new(4, PrivilegeLevel::Ring3));
        }
        //entry.set_privilege_level(PrivilegeLevel::Ring3);

        unsafe {
            //entry.set_code_selector(SegmentSelector::new(0x08, PrivilegeLevel::Ring0));
        }

        unsafe {
            self.descriptor_table.load_unsafe();
        }

        Some(free_vector)
    }

    pub fn enable_timer(&mut self, entry_point: InterruptEntryPoint, duration: Duration) {
        match self.r#type {
            LocalInterruptManagerType::XAPIC {
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

    pub fn enable_oneshot(
        &mut self,
        entry_point: InterruptEntryPoint,
        duration: Duration,
        logger: &crate::kernel::logger::KernelLogger,
    ) {
        match self.r#type {
            LocalInterruptManagerType::XAPIC {
                base,
                ticks_per_millisecond,
            } => unsafe {
                let vector = self.allocate(entry_point).unwrap();

                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());

                xapic.enable_oneshot(
                    vector,
                    (duration.as_millis() as u32) * ticks_per_millisecond,
                    logger,
                );
            },
            _ => todo!(),
        }
    }

    pub fn reset_oneshot(&self, duration: Duration) {
        match self.r#type {
            LocalInterruptManagerType::XAPIC {
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
            LocalInterruptManagerType::XAPIC { base, .. } => unsafe {
                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());
                xapic.signal_end_of_interrupt();
            },
            _ => todo!(),
        }
    }
}

pub struct KernelInterruptManager {
    local_interrupt_managers: Mutex<BTreeMap<ExecutionUnitIdentifier, LocalInterruptManager>>,
}

impl KernelInterruptManager {
    pub fn new() -> Self {
        Self {
            local_interrupt_managers: Mutex::new(BTreeMap::default()),
        }
    }

    pub fn bootstrap_local_interrupt_manager(
        &mut self,
        local_interrupt_manager: LocalInterruptManager,
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

    pub fn reset_oneshot(&self, duration: Duration) {
        self.local_interrupt_managers
            .lock()
            .get(&0)
            .unwrap()
            .reset_oneshot(duration);
    }
}
