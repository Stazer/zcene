use crate::actor::unprivileged::ActorUnprivilegedAddress;
use zcene_core::actor::{Actor, ActorCommonHandleContext, ActorEnvironment, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedEnvironment;

impl ActorEnvironment for ActorUnprivilegedEnvironment {
    type Address<A>
        = ActorUnprivilegedAddress<A>
    where
        A: Actor<Self>;

    type CreateContext<'a> = ();

    type HandleContext<'a, M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;

    type DestroyContext<'a> = ();
}
