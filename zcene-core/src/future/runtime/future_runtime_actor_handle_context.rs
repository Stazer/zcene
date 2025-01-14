use ztd::Constructor;
use crate::actor::{ActorContextMessageProvider, ActorMessage};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct FutureRuntimeActorHandleContext<M>
where
    M: ActorMessage,
{
    message: M,
}

impl<M> ActorContextMessageProvider<M> for FutureRuntimeActorHandleContext<M>
where
    M: ActorMessage,
{
    fn message(&self) -> &M {
        &self.message
    }
}
