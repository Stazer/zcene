use crate::driver::acpi::hpet::HpetRegisters;
use crate::time::{Timer, TimerInstant};
use core::time::Duration;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub const HPET_GENERAL_CONFIGURATION_ENABLE_CNF_BIT: u64 = 0x1;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct Hpet<'a> {
    registers: &'a mut HpetRegisters,
}

impl<'a> Hpet<'a> {
    pub fn counter(&self) -> u64 {
        self.registers.main_counter_value().read()
    }

    pub fn is_enabled(&self) -> bool {
        self.registers.general_configuration().read() & HPET_GENERAL_CONFIGURATION_ENABLE_CNF_BIT
            == 1
    }

    pub fn is_disabled(&self) -> bool {
        self.registers.general_configuration().read() & HPET_GENERAL_CONFIGURATION_ENABLE_CNF_BIT
            == 0
    }

    pub fn enable(&mut self) {
        self.registers
            .general_configuration_mut()
            .update(|old_val_ref| old_val_ref | HPET_GENERAL_CONFIGURATION_ENABLE_CNF_BIT);
    }

    pub fn disable(&mut self) {
        self.registers
            .general_configuration_mut()
            .update(|old_val_ref| old_val_ref & !HPET_GENERAL_CONFIGURATION_ENABLE_CNF_BIT);
    }

    pub fn counter_tick_in_femtoseconds(&self) -> u32 {
        (self.registers.general_capabilities_and_id().read() >> 32) as u32
    }
}

impl<'a> Timer for Hpet<'a> {
    fn now(&self) -> TimerInstant {
        TimerInstant::new(self.counter().try_into().unwrap_or(u32::MAX))
    }

    fn duration_between(&self, start: TimerInstant, end: TimerInstant) -> Duration {
        let ticks = end.value().saturating_sub(start.value());

        Duration::from_nanos(
            u64::from(ticks) * u64::from(self.counter_tick_in_femtoseconds()) / 10u64.pow(6),
        )
    }
}
