use crate::actor::root::ActorRootEnvironment;
use crate::actor::unprivileged::ActorUnprivilegedEnvironmentPerspective;
use zcene_core::actor::{ActorMessage, ActorRecipient};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedEnvironmentPrivilegedPerspective;

impl ActorUnprivilegedEnvironmentPerspective for ActorUnprivilegedEnvironmentPrivilegedPerspective {
    type Handle<M>
        = ActorRecipient<M, ActorRootEnvironment>
    where
        M: ActorMessage;
}
