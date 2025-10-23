use crate::actor::{
    ActorCommonBounds, ActorCreateError, ActorDestroyError, ActorEnvironment, ActorFuture,
    ActorHandleError, ActorMessage,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Actor<E>: ActorCommonBounds + Sized
where
    E: ActorEnvironment,
{
    type Message: ActorMessage;

    fn create<'a>(
        &'a mut self,
        _context: E::CreateContext<'a>,
    ) -> impl ActorFuture<'a, Result<(), ActorCreateError>> {
        async { Ok(()) }
    }

    fn handle<'a>(
        &mut self,
        _context: E::HandleContext<'a, Self::Message>,
    ) -> impl ActorFuture<'a, Result<(), ActorHandleError>> {
        async { Ok(()) }
    }

    fn destroy<'a>(
        self,
        _context: E::DestroyContext<'a>,
    ) -> impl ActorFuture<'a, Result<(), ActorDestroyError>> {
        async { Ok(()) }
    }
}
