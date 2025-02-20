use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorHandler};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorUnprivilegedExecutorHandleState<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    message: A::Message,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
