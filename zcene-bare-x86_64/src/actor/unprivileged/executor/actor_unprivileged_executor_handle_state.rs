use crate::actor::ActorUnprivilegedHandler;
use alloc::boxed::Box;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorEnvironment, ActorMessage};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorUnprivilegedExecutorHandleState<A, E, M>
where
    A: Actor<ActorUnprivilegedHandler>,
    E: ActorEnvironment,
    M: ActorMessage,
{
    actor: Box<A>,
    message: M,
    #[Constructor(default)]
    marker: PhantomData<E>,
}
