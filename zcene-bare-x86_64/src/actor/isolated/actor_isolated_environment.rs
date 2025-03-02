use zcene_core::actor::{ActorEnvironment, ActorCommonHandleContext, ActorMessage, Actor};
use crate::actor::ActorIsolatedAddress;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorIsolatedEnvironment;

impl ActorEnvironment for ActorIsolatedEnvironment{
    type Address<A>
        = ActorIsolatedAddress<A>
    where
        A: Actor<Self>;
    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();
}
