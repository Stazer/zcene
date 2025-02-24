use crate::actor::ActorUnprivilegedStageExecutorContext;
use alloc::boxed::Box;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorHandler};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorUnprivilegedExecutorDestroyState<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: Box<A>,
    context: Option<ActorUnprivilegedStageExecutorContext>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
