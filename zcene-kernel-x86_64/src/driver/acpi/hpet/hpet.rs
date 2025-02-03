use crate::driver::acpi::hpet::{HpetInstant, HpetRegisters};
use zcene_kernel::time::Timer;
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
    type Instant = HpetInstant;

    fn now(&self) -> Self::Instant {
        HpetInstant::new(self.counter(), self.counter_tick_in_femtoseconds().into())
    }
}
