use crate::actor::ActorIsolationAddress;
use zcene_core::actor::{Actor, ActorCommonHandleContext, ActorEnvironment, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolationEnvironment;

impl ActorEnvironment for ActorIsolationEnvironment {
    type Address<A>
        = ActorIsolationAddress<A>
    where
        A: Actor<Self>;
    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();
}
