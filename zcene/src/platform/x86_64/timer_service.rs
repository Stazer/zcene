use crate::common::memory::address::PhysicalMemoryAddress;
use crate::common::time::{AtomicTimer, Timer, TimerInstant};
use crate::driver::acpi::hpet::{Hpet, HpetRegisters};
use crate::platform::x86_64::{KernelMemoryManagerAcpiHandler, X86_64MemoryManager};
use acpi::{AcpiTables, HpetInfo};
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum TimerService {
    Atomic(AtomicTimer),
    Hpet(Hpet<'static>),
}

impl Timer for TimerService {
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

impl TimerService {
    pub fn new() -> Self {
        Self::Atomic(AtomicTimer::default())
    }

    /// Looks up the HPET via the ACPI tables reachable from `rsdp_address` and, if present,
    /// enables it and returns a `Timer` backed by real elapsed time. Returns `None` if the
    /// tables can't be parsed or the machine has no HPET, in which case the caller should
    /// fall back to `Self::new()`.
    pub fn from_hpet(rsdp_address: u64, memory: &X86_64MemoryManager) -> Option<Self> {
        let handler = KernelMemoryManagerAcpiHandler::new(memory);
        let tables = unsafe { AcpiTables::from_rsdp(handler, rsdp_address as usize) }.ok()?;
        let hpet_info = HpetInfo::new(&tables).ok()?;

        let registers = memory
            .translate_physical_memory_address(PhysicalMemoryAddress::from(
                hpet_info.base_address as u64,
            ))
            .cast_mut::<HpetRegisters>();

        let mut hpet = Hpet::new(unsafe { &mut *registers });

        if hpet.is_disabled() {
            hpet.enable();
        }

        Some(Self::Hpet(hpet))
    }
}
