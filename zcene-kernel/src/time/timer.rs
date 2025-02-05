use crate::time::TimerInstant;
use core::time::Duration;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Timer {
    fn now(&self) -> TimerInstant;

    fn duration_between(&self, start: TimerInstant, end: TimerInstant) -> Duration;
}
