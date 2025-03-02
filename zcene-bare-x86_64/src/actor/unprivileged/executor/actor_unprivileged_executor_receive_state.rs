use crate::actor::ActorUnprivilegedHandler;
use alloc::boxed::Box;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorEnvironment};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorUnprivilegedExecutorReceiveState<A, E>
where
    A: Actor<E> + Actor<ActorUnprivilegedHandler>,
    E: ActorEnvironment,
{
    actor: Box<A>,
    #[Constructor(default)]
    marker: PhantomData<E>,
}
