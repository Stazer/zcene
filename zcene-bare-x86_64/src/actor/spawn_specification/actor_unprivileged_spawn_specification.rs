use core::num::NonZero;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Default, Method)]
#[Method(accessors)]
pub struct ActorUnprivilegedSpawnSpecification {
    deadline_in_milliseconds: Option<NonZero<usize>>,
}
