use crate::actor::ActorUnprivilegedHandler;
use crate::actor::ActorUnprivilegedStageExecutorContext;
use alloc::boxed::Box;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorEnvironment};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorUnprivilegedExecutorCreateState<A, E>
where
    A: Actor<E> + Actor<ActorUnprivilegedHandler>,
    E: ActorEnvironment,
{
    actor: Box<A>,
    context: Option<ActorUnprivilegedStageExecutorContext>,
    #[Constructor(default)]
    marker: PhantomData<E>,
}
