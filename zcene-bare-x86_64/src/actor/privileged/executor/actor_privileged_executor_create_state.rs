use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorEnvironment};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorPrivilegedExecutorCreateState<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    actor: A,
    #[Constructor(default)]
    marker: PhantomData<E>,
}
