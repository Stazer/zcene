use zcene_kernel::time::{Timer, TimerInstant};
use zcene_kernel::memory::address::{PhysicalMemoryAddress};
use crate::driver::xapic::{XApicRegisters};
use ztd::Constructor;
use core::time::Duration;
use x86::msr::rdmsr;
use x86::msr::APIC_BASE;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct XApic<'a> {
    registers: &'a mut XApicRegisters,
}


impl<'a> XApic<'a> {
    pub fn base_address() -> PhysicalMemoryAddress {
        PhysicalMemoryAddress::from(
            unsafe { rdmsr(APIC_BASE) & 0xFFFFF000 }
        )
    }

    pub fn set_spurious_interrupt_vector(&mut self, vector: u8) {
        let svr: u32 = 1 << 8 | 15;
        self.registers
            .spurious_interrupt_vector_mut()
            .write(1 << 8 | vector as u32);
    }

    pub fn calibrate<T>(&mut self, timer: &T, duration: Duration) -> u32
    where
        T: Timer,
    {
        self.registers.timer_initial_count_mut().write(u32::MAX);
        self.registers.timer_divide_mut().write(0b1011);
        self.registers.lvt_timer_mut().write(0x10000);

        let start = timer.now();

        while timer.duration_between(start, timer.now()) < duration {}

        u32::MAX - self.registers.timer_current_count().read()
    }

    pub fn enable_timer(&mut self, vector: u8, ticks: u32) {
        self.registers.timer_initial_count_mut().write(ticks);
        self.registers.timer_divide_mut().write(0b1011);

        let mode_periodic = 0x20000;
        let mask = 0;
        let value = mode_periodic | vector as u32 | mask;

        self.registers.lvt_timer_mut().write(value);
    }
}
