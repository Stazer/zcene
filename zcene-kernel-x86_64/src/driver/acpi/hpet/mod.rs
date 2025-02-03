use zcene_kernel::common::volatile::{ReadVolatile, ReadWriteVolatile};
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Method)]
#[repr(C)]
pub struct HpetRegisters {
    #[Method(accessor)]
    general_capabilities_and_id: ReadVolatile<u64>,
    _unused0: u64,
    #[Method(accessor, mutator)]
    general_configuration: ReadWriteVolatile<u64>,
    _unused1: u64,
    #[Method(accessor, mutator)]
    general_interrupt_status: ReadWriteVolatile<u64>,
    _unused2: [u64; 235],
    #[Method(accessor, mutator)]
    main_counter_value: ReadWriteVolatile<u64>,
    _unused3: u64,
}

impl HpetRegisters {
    pub fn counter(&self) -> u64 {
        self.main_counter_value.read()
    }

    pub fn enable(&mut self) {
        self.general_configuration
            .update(|old_val_ref| old_val_ref | 0x1);
    }

    pub fn disable(&mut self) {
        self.general_configuration
            .update(|old_val_ref| old_val_ref & !0x1);
    }

    pub fn counter_period_in_femtoseconds(&self) -> u32 {
        (self.general_capabilities_and_id.read() >> 32) as u32
    }
}

pub struct HpetClock {
    registers: &'static mut HpetRegisters,
}

impl HpetClock {
    pub fn counter(&self) -> u64 {
        self.registers.main_counter_value().read()
    }

    pub fn enable(&mut self) {
        self.registers
            .general_configuration_mut()
            .update(|old_val_ref| old_val_ref | 0x1);
    }

    pub fn disable(&mut self) {
        self.registers
            .general_configuration_mut()
            .update(|old_val_ref| old_val_ref & !0x1);
    }

    pub fn counter_period_in_femtoseconds(&self) -> u32 {
        (self.registers.general_capabilities_and_id().read() >> 32) as u32
    }
}

pub trait Clock {
    type Instant;

    fn now(&self) -> Self::Instant;
}

pub struct HpetInstant {
    value: usize,
}

impl Clock for HpetClock {
    type Instant:
}
