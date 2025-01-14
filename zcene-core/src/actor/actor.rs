use crate::actor::{
    ActorCommonBounds, ActorCreateError, ActorDestroyError, ActorFuture, ActorHandleError,
    ActorHandler, ActorMessage,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Actor<H>: ActorCommonBounds + Sized
where
    H: ActorHandler,
{
    type Message: ActorMessage;

    fn create(
        &mut self,
        _context: H::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        async { Ok(()) }
    }

    fn handle(
        &mut self,
        _context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async { Ok(()) }
    }

    fn destroy(
        self,
        _context: H::DestroyContext,
    ) -> impl ActorFuture<'static, Result<(), ActorDestroyError>> {
        async { Ok(()) }
    }
}
