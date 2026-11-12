use crate::actor::isolated::ActorIsolatedEnvironmentPerspective;
use crate::actor::root::ActorRootEnvironment;
use zcene_core::actor::{ActorMessage, ActorRecipient};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedEnvironmentPrivilegedPerspective;

impl ActorIsolatedEnvironmentPerspective for ActorIsolatedEnvironmentPrivilegedPerspective {
    type Handle<M>
        = ActorRecipient<M, ActorRootEnvironment>
    where
        M: ActorMessage;
}
