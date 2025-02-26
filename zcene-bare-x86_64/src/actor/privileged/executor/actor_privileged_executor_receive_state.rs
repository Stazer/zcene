use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorHandler};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorPrivilegedExecutorReceiveState<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    #[Constructor(default)]
    marker: PhantomData<H>,
}
