use core::time::Duration;
use zcene_kernel::time::TimerInstant;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Constructor, Debug)]
pub struct HpetInstant {
    counter: u64,
    counter_tick_in_femtoseconds: u64,
}

impl TimerInstant for HpetInstant {
    fn duration_since(&self, start: &Self) -> Duration {
        Duration::from_nanos(
            (self.counter - start.counter) * self.counter_tick_in_femtoseconds / 10u64.pow(6),
        )
    }
}
