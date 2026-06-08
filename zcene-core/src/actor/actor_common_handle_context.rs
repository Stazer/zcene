use crate::actor::ActorMessage;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorCommonHandleContext<M>
where
    M: ActorMessage,
{
    message: M,
}

impl<M> ActorCommonHandleContext<M>
where
    M: ActorMessage,
{
    pub fn message(&self) -> &M {
        &self.message
    }

    pub fn into_message(self) -> M {
        self.message
    }
}
