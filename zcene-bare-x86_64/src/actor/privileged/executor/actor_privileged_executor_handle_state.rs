use core::marker::PhantomData;
use zcene_core::actor::{Actor, ActorEnvironment};
use ztd::{Constructor, Inner};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Inner)]
pub struct ActorPrivilegedExecutorHandleState<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    actor: A,
    message: A::Message,
    #[Constructor(default)]
    marker: PhantomData<E>,
}
