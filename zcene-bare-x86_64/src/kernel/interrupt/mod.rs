use crate::driver::xapic::XApic;
use crate::kernel::memory::{KernelMemoryManager};
use core::time::Duration;
use pic8259::ChainedPics;
use x86::cpuid::CpuId;
use crate::driver::xapic::{XApicRegisters};
use alloc::boxed::Box;
use x86_64::structures::idt::InterruptDescriptorTable;
use zcene_bare::synchronization::Mutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;

pub enum LocalInterruptManagerType {
    PIC,
    XAPIC { base: *mut u32 },
    X2APIC,
}

unsafe impl Sync for LocalInterruptManagerType {}
unsafe impl Send for LocalInterruptManagerType {}

impl LocalInterruptManagerType {}

pub struct LocalInterruptManager {
    r#type: LocalInterruptManagerType,
    descriptor_table: Mutex<Box<InterruptDescriptorTable>>,
}

impl LocalInterruptManager {
    pub fn new(memory_manager: &KernelMemoryManager) -> Self {
        let descriptor_table = Box::new(InterruptDescriptorTable::new());

        unsafe {
            descriptor_table.load_unsafe();
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

            return Self {
                r#type: LocalInterruptManagerType::XAPIC {
                    base: memory_manager
                        .translate_physical_memory_address(XApic::base_address())
                        .cast_mut::<u32>(),
                },
                descriptor_table: Mutex::new(descriptor_table),
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

    pub fn enable_timer(&self, vector: u8, duration: Duration) {
        match self.r#type {
            LocalInterruptManagerType::XAPIC { base } => unsafe {
                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());
                let ticks = xapic.calibrate(crate::kernel::Kernel::get().timer(), duration);
                xapic.enable_timer(vector, ticks);
            },
            _ => todo!(),
        }
    }
}

use alloc::collections::BTreeMap;
use crate::architecture::ExecutionUnitIdentifier;

pub struct KernelInterruptManager {
    local_interrupt_managers: Mutex<BTreeMap<ExecutionUnitIdentifier, LocalInterruptManager>>,
}

impl KernelInterruptManager {
    pub fn new(
        memory_manager: &KernelMemoryManager,
    ) -> Self {
        Self {
            local_interrupt_managers: Mutex::new(BTreeMap::default()),
        }
    }

    /*pub fn local_interrupt_manager(&self) -> LocalInterruptManager {
        todo!()
    }*/
}

    /*fn initialize_interrupts(&self) -> Result<(), KernelInitializeError> {
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
            .map_err(KernelInitializeError::BuildLocalApic)?;

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

            for i in EXTERNAL_INTERRUPTS_START..(u8::MAX as usize) {
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

        use crate::kernel::LocalInterruptManager;

        let cpu_id = CpuId::new();
        let feature_info = cpu_id.get_feature_info().unwrap();

        let r#type = LocalInterruptManager::new(self.memory_manager());

        r#type.enable_timer(0x20, core::time::Duration::from_millis(100));

        /*unsafe {
            let mut apic = IoApic::new((self.physical_memory_offset + 0xFEC00000) as _);
            apic.enable(1, 0);
        }*/

        Ok(())
        //Ok(local_apic)
    }*/
