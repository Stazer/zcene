use crate::time::{TimerInstant};
use ztd::Constructor;
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct AtomicTimerInstant {
    counter: u32,
    duration: Duration
}

impl TimerInstant for AtomicTimerInstant {
    fn duration_since(&self, start: &Self) -> Duration {
        (self.duration * self.counter) - (start.duration * start.counter)
    }
}
