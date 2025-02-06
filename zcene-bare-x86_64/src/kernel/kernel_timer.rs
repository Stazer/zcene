use crate::driver::acpi::hpet::{Hpet, HpetRegisters};
use crate::kernel::memory::{KernelMemoryManager, KernelMemoryManagerAcpiHandler};
use acpi::{AcpiTables, HpetInfo};
use core::time::Duration;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::time::{AtomicTimer, Timer, TimerInstant};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum KernelTimer<'a> {
    Atomic(AtomicTimer),
    Hpet(Hpet<'a>),
}

impl<'a> KernelTimer<'a> {
    pub fn new(
        memory_manager: &KernelMemoryManager,
        rsdp_address: Option<PhysicalMemoryAddress>,
    ) -> Self {
        rsdp_address
            .map(|rsdp_address| Self::new_hpet(memory_manager, rsdp_address))
            .flatten()
            .unwrap_or_else(Self::new_atomic)
    }

    fn new_hpet(
        memory_manager: &KernelMemoryManager,
        rsdp_address: PhysicalMemoryAddress,
    ) -> Option<Self> {
        let acpi_handler = KernelMemoryManagerAcpiHandler::new(memory_manager);

        let acpi_tables =
            unsafe { AcpiTables::from_rsdp(acpi_handler, rsdp_address.as_usize()).ok()? };

        let hpet_address = memory_manager.translate_physical_memory_address(
            PhysicalMemoryAddress::new(HpetInfo::new(&acpi_tables).ok()?.base_address),
        );

        let mut hpet = Hpet::new(unsafe { hpet_address.cast_mut::<HpetRegisters>().as_mut()? });

        hpet.enable();

        Some(Self::Hpet(hpet))
    }

    fn new_atomic() -> Self {
        Self::Atomic(AtomicTimer::new(Duration::from_millis(10)))
    }
}

impl<'a> Timer for KernelTimer<'a> {
    fn now(&self) -> TimerInstant {
        match self {
            Self::Atomic(atomic) => atomic.now(),
            Self::Hpet(hpet) => hpet.now(),
        }
    }

    fn duration_between(&self, start: TimerInstant, end: TimerInstant) -> Duration {
        match self {
            Self::Atomic(atomic) => atomic.duration_between(start, end),
            Self::Hpet(hpet) => hpet.duration_between(start, end),
        }
    }
}
