use crate::time::{Timer, AtomicTimerInstant};
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct AtomicTimer {
    counter: AtomicU32,
    duration: Duration,
}

impl AtomicTimer {
   pub fn new(
       duration: Duration,
   ) -> Self {
       Self {
           counter: AtomicU32::default(),
           duration,
       }
   }
}

impl Timer for AtomicTimer {
    type Instant = AtomicTimerInstant;

    fn now(&self) -> Self::Instant {
        AtomicTimerInstant::new(
            self.counter.fetch_add(1, Ordering::SeqCst),
            self.duration,
        )
    }
}
