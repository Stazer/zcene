use crate::actor::{ActorContextMessageProvider, ActorMessage};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorCommonHandleContext<M>
where
    M: ActorMessage,
{
    message: M,
}

impl<M> ActorContextMessageProvider<M> for ActorCommonHandleContext<M>
where
    M: ActorMessage,
{
    fn message(&self) -> &M {
        &self.message
    }
}
