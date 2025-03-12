use crate::actor::{
    ActorCommonBounds, ActorCreateError, ActorDestroyError, ActorEnvironment, ActorFuture,
    ActorHandleError, ActorMessage,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

use core::future::Future;

pub trait Actor<E>: ActorCommonBounds + Sized
where
    E: ActorEnvironment,
{
    type Message: ActorMessage;

    fn create(
        &mut self,
        _context: E::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        async { Ok(()) }
    }

    fn create2<'a>(
        &'a mut self,
        _context: E::CreateContext2<'a>,
    ) -> impl ActorFuture<'a, Result<(), ActorCreateError>> {
        async { Ok(()) }
    }

    fn handle(
        &mut self,
        _context: E::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        async { Ok(()) }
    }

    fn destroy(
        self,
        _context: E::DestroyContext,
    ) -> impl ActorFuture<'static, Result<(), ActorDestroyError>> {
        async { Ok(()) }
    }
}
