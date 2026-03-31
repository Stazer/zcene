use core::time::Duration;
use crate::common::time::{AtomicTimer, Timer, TimerInstant};
use crate::driver::acpi::hpet::{Hpet, HpetRegisters};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum ActorRootEnvironmentTimerService {
    Atomic(AtomicTimer),
    Hpet(Hpet<'static>),
}

impl Timer for ActorRootEnvironmentTimerService {
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

impl ActorRootEnvironmentTimerService {
    pub fn new() -> ActorRootEnvironmentTimerService {
        ActorRootEnvironmentTimerService::Atomic(AtomicTimer::default())
    }
}
