use crate::actor::ActorUnprivilegedHandler;
use alloc::boxed::Box;
use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorHandler};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorUnprivilegedExecutorHandleState<A, H>
where
    A: Actor<H> + Actor<ActorUnprivilegedHandler>,
    H: ActorHandler,
{
    actor: Box<A>,
    message: <A as Actor<H>>::Message,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
