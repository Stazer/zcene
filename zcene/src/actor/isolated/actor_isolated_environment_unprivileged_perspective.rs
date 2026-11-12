use crate::actor::isolated::ActorIsolatedEnvironmentPerspective;
use zcene_core::actor::ActorMessage;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedEnvironmentUnprivilegedPerspective;

impl ActorIsolatedEnvironmentPerspective for ActorIsolatedEnvironmentUnprivilegedPerspective {
    type Handle<M>
        = usize
    where
        M: ActorMessage;
}
