use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Copy, Clone, Debug, Method)]
#[Method(all)]
#[repr(transparent)]
pub struct TimerInstant {
    value: u32,
}
