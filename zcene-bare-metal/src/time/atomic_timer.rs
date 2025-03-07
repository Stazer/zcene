use crate::time::{Timer, TimerInstant};
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct AtomicTimer {
    counter: AtomicU32,
    duration: Duration,
}

impl AtomicTimer {
    pub fn new(duration: Duration) -> Self {
        Self {
            counter: AtomicU32::default(),
            duration,
        }
    }
}

impl Timer for AtomicTimer {
    fn now(&self) -> TimerInstant {
        TimerInstant::new(self.counter.fetch_add(1, Ordering::SeqCst))
    }

    fn duration_between(&self, start: TimerInstant, end: TimerInstant) -> Duration {
        let factor = end.value().saturating_sub(start.value());

        self.duration * factor
    }
}
