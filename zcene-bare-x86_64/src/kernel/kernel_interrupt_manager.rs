pub struct KernelInterruptManager {

}

use crate::architecture::FRAME_SIZE;
use crate::driver::xapic::XApic;
use crate::kernel::memory::KernelMemoryManager;
use core::mem::size_of;
use core::ptr::{read_volatile, write_volatile};
use core::slice::from_raw_parts_mut;
use core::time::Duration;
use pic8259::ChainedPics;
use x86::cpuid::CpuId;
use x86::msr::{rdmsr, APIC_BASE};
use x86::time::rdtsc;

////////////////////////////////////////////////////////////////////////////////////////////////////

const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;

pub enum LocalInterruptManagerType {
    PIC,
    XAPIC { base: *mut u32 },
    X2APIC,
}

impl LocalInterruptManagerType {}

pub struct LocalInterruptManager {
    r#type: LocalInterruptManagerType,
}

impl LocalInterruptManager {
    pub fn new(memory_manager: &KernelMemoryManager) -> Self {
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
                use crate::driver::xapic::{XApic, XApicRegisters};

                let mut xapic = XApic::new((base as *mut XApicRegisters).as_mut().unwrap());
                let ticks = xapic.calibrate(crate::kernel::Kernel::get().timer(), duration);
                xapic.enable_timer(vector, ticks);
            },
            _ => todo!(),
        }
    }
}

use ztd::Constructor;

pub struct InterruptManager {}

impl InterruptManager {
    pub fn new() -> Self {
        Self {}
    }

    /*pub fn local_interrupt_manager(&self) -> LocalInterruptManager {
        todo!()
    }*/
}
