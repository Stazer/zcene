use crate::actor::unprivileged::ActorUnprivilegedEnvironmentPerspective;
use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedEnvironmentUnprivilegedPerspective;

impl ActorUnprivilegedEnvironmentPerspective for ActorUnprivilegedEnvironmentUnprivilegedPerspective {
    type Handle<M>
        = usize
    where
        M: ActorMessage;
}
