use crate::actor::root::ActorRootEnvironment;
use zcene_core::actor::{ActorMessage, ActorRecipient};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorUnprivilegedPerspective: Send + Sync + 'static {

    type Handle<M>: Clone + Send + Sync + 'static
    where
        M: ActorMessage;
}

pub struct ActorUnprivilegedKernelPerspective;

impl ActorUnprivilegedPerspective for ActorUnprivilegedKernelPerspective {
    type Handle<M>
        = ActorRecipient<M, ActorRootEnvironment>
    where
        M: ActorMessage;
}

pub struct ActorUnprivilegedUserPerspective;

impl ActorUnprivilegedPerspective for ActorUnprivilegedUserPerspective {
    type Handle<M>
        = usize
    where
        M: ActorMessage;
}
