use x86::cpuid::CpuId;
use x86::msr::{rdmsr, APIC_BASE};
use core::slice::from_raw_parts_mut;
use pic8259::ChainedPics;
use core::mem::size_of;
use core::ptr::{write_volatile, read_volatile};
use x86::apic::xapic::XAPIC_LVT_TIMER;
use x86::apic::xapic::XAPIC_TIMER_DIV_CONF;
use x86::apic::xapic::XAPIC_TIMER_INIT_COUNT;
use x86::apic::xapic::{XAPIC_TIMER_CURRENT_COUNT, XAPIC_SVR};
use x86::time::rdtsc;
use crate::architecture::FRAME_SIZE;
use core::time::Duration;

const PARENT_PIC_OFFSET: u8 = EXTERNAL_INTERRUPTS_START;
const CHILD_PIC_OFFSET: u8 = PARENT_PIC_OFFSET + 0x8;
const EXTERNAL_INTERRUPTS_START: u8 = 0x20;

pub struct LocalAPIC {
    base: &'static mut [u32],
}

impl LocalAPIC {

}

pub enum LocalInterruptManagerType {
    PIC,
    XAPIC {
        base: *mut u32,
    },
    X2APIC,
}

impl LocalInterruptManagerType {
}

pub struct LocalInterruptManager {
    r#type: LocalInterruptManagerType,
}

impl LocalInterruptManager {
    pub fn new(physical_memory_offset: u64) -> Self {
        let cpu_id = CpuId::new();
        let feature_info = cpu_id.get_feature_info();

        if feature_info.as_ref().map(|f| f.has_apic()).unwrap_or_default() {
            unsafe {
                let mut pic = ChainedPics::new(PARENT_PIC_OFFSET, CHILD_PIC_OFFSET);
                pic.disable();
            }

            let apic_base = unsafe {
                rdmsr(APIC_BASE) & 0xFFFFF000
            };

            return Self {
                r#type: LocalInterruptManagerType::XAPIC {
                    base: (apic_base + physical_memory_offset) as *mut u32,
                }
            };
        }

        if feature_info.as_ref().map(|f| f.has_x2apic()).unwrap_or_default() {
            /*let mut local_apic = LocalApicBuilder::new()
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

            todo!()
        }

        todo!()
    }

    pub fn enable_timer(
        &self,
        vector_entry: u8,
        duration: Duration,
    ) {
        let hpet = crate::kernel::Kernel::get().hpet();

        match self.r#type {
            LocalInterruptManagerType::XAPIC { base } => {
                unsafe {
                    let svr: u32 = 1 << 8 | 15;
                    self.write_xapic_register(&base, XAPIC_SVR, svr);

                    let counter_period_in_femtoseconds = hpet.counter_period_in_femtoseconds() as u64;

                    self.write_xapic_register(&base, XAPIC_TIMER_INIT_COUNT, u32::MAX);
                    self.write_xapic_register(&base, XAPIC_TIMER_DIV_CONF, 0b1011);
                    self.write_xapic_register(&base, XAPIC_LVT_TIMER, 0x10000);

                    let start = hpet.counter();

                    loop {
                        let current = hpet.counter();

                        if u128::from((hpet.counter() - start) * counter_period_in_femtoseconds) >= duration.as_millis() * 10u128.pow(12) {
                            break
                        }
                    }

                    let now = self.read_xapic_register(&base, XAPIC_TIMER_CURRENT_COUNT);
                    let total = u32::MAX - now;

                    self.write_xapic_register(&base, XAPIC_TIMER_INIT_COUNT, total);
                    self.write_xapic_register(&base, XAPIC_TIMER_DIV_CONF, 0b1011);
                    let vector = vector_entry as u32;
                    let mode_periodic = 0x20000;
                    let mask = 0;
                    let value = mode_periodic | vector | mask;
                    self.write_xapic_register(&base, XAPIC_LVT_TIMER, value);
                }
            }
            _ => todo!(),
        }
    }

    unsafe fn write_xapic_register(&self, base: &*mut u32, register: u32, value: u32) {
        write_volatile(base.add((register as usize) / size_of::<u32>()), value);
    }

    unsafe fn read_xapic_register(&self, base: &*mut u32, register: u32) -> u32 {
        read_volatile(base.add((register as usize) / size_of::<u32>()))
    }
}

use ztd::Constructor;

#[derive(Constructor)]
pub struct InterruptManager {
    physical_memory_offset: u64,
}

impl InterruptManager {
    pub fn local_interrupt_manager(&self) -> LocalInterruptManager {
        LocalInterruptManager::new(self.physical_memory_offset)
    }
}
