use crate::common::memory::address::PhysicalMemoryAddress;
use crate::common::time::Timer;
use crate::driver::xapic::XApicRegisters;
use core::time::Duration;
use x86::msr::APIC_BASE;
use x86::msr::rdmsr;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct XApic<'a> {
    registers: &'a mut XApicRegisters,
}

impl<'a> XApic<'a> {
    pub fn base_address() -> PhysicalMemoryAddress {
        PhysicalMemoryAddress::from(unsafe { rdmsr(APIC_BASE) & 0xFFFFF000 })
    }

    pub fn set_spurious_interrupt_vector(&mut self, vector: u8) {
        let _svr: u32 = 1 << 8 | 15;
        self.registers
            .spurious_interrupt_vector_mut()
            .write(1 << 8 | vector as u32);
    }

    pub fn signal_end_of_interrupt(&mut self) {
        self.registers.eoi_mut().write(0);
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

        self.registers
            .lvt_timer_mut()
            .write(0x20000 | (vector as u32));
    }

    pub fn enable_oneshot(&mut self, vector: u8, ticks: u32) {
        self.registers.timer_initial_count_mut().write(ticks);
        self.registers.timer_divide_mut().write(0);
        //0b1011);

        self.registers
            .lvt_timer_mut()
            .write(0x40000 | (vector as u32));
    }

    pub fn reset_oneshot(&mut self, ticks: u32) {
        self.registers.timer_initial_count_mut().write(ticks);
    }

    pub fn reset_deadline(&mut self) {}

    /// Broadcasts an INIT IPI to every logical core except the sender, as the first step of
    /// the Intel MP startup sequence (INIT-SIPI-SIPI).
    pub fn send_init_ipi_all_excluding_self(&mut self) {
        self.registers.interrupt_command_high_mut().write(0);
        self.registers
            .interrupt_command_low_mut()
            .write(0x000C4500);
    }

    /// Broadcasts a Startup IPI (SIPI) to every logical core except the sender, pointing at
    /// the real-mode entry point `vector * 0x1000` (so `vector` must address a frame below 1 MiB).
    pub fn send_startup_ipi_all_excluding_self(&mut self, vector: u8) {
        self.registers.interrupt_command_high_mut().write(0);
        self.registers
            .interrupt_command_low_mut()
            .write(0x000C4600 | vector as u32);
    }
}
