use crate::actor::{
    Actor, ActorCommonBounds, ActorCreateError, ActorDestroyError, ActorFuture, ActorHandleError,
    ActorHandler, ActorMessage,
};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct InlineActor<H, M, CF, CR, HF, HR, DF, DR>
where
    H: ActorHandler,
    M: ActorMessage,
    CF: FnMut(H::CreateContext) -> CR + ActorCommonBounds,
    CR: for<'a> ActorFuture<'a, Result<(), ActorCreateError>> + ActorCommonBounds,
    HF: FnMut(H::HandleContext<M>) -> HR + ActorCommonBounds,
    HR: for<'a> ActorFuture<'a, Result<(), ActorHandleError>> + ActorCommonBounds,
    DF: FnOnce(H::DestroyContext) -> DR + ActorCommonBounds,
    DR: for<'a> ActorFuture<'a, Result<(), ActorDestroyError>> + ActorCommonBounds,
{
    create_function: CF,
    handle_function: HF,
    destroy_function: DF,
    types: PhantomData<(H, M, CR, HR, DR)>,
}

impl<H, M, CF, CR, HF, HR, DF, DR> InlineActor<H, M, CF, CR, HF, HR, DF, DR>
where
    H: ActorHandler,
    M: ActorMessage,
    CF: FnMut(H::CreateContext) -> CR + ActorCommonBounds,
    CR: for<'a> ActorFuture<'a, Result<(), ActorCreateError>> + ActorCommonBounds,
    HF: FnMut(H::HandleContext<M>) -> HR + ActorCommonBounds,
    HR: for<'a> ActorFuture<'a, Result<(), ActorHandleError>> + ActorCommonBounds,
    DF: FnOnce(H::DestroyContext) -> DR + ActorCommonBounds,
    DR: for<'a> ActorFuture<'a, Result<(), ActorDestroyError>> + ActorCommonBounds,
{
    pub fn new(create_function: CF, handle_function: HF, destroy_function: DF) -> Self {
        Self {
            create_function,
            handle_function,
            destroy_function,
            types: PhantomData::<(H, M, CR, HR, DR)>,
        }
    }
}

impl<H, M, CF, CR, HF, HR, DF, DR> Actor<H> for InlineActor<H, M, CF, CR, HF, HR, DF, DR>
where
    H: ActorHandler,
    M: ActorMessage,
    CF: FnMut(H::CreateContext) -> CR + ActorCommonBounds,
    CR: for<'a> ActorFuture<'a, Result<(), ActorCreateError>> + ActorCommonBounds,
    HF: FnMut(H::HandleContext<M>) -> HR + ActorCommonBounds,
    HR: for<'a> ActorFuture<'a, Result<(), ActorHandleError>> + ActorCommonBounds,
    DF: FnOnce(H::DestroyContext) -> DR + ActorCommonBounds,
    DR: for<'a> ActorFuture<'a, Result<(), ActorDestroyError>> + ActorCommonBounds,
{
    type Message = M;

    fn create(
        &mut self,
        context: H::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        (self.create_function)(context)
    }

    fn handle(
        &mut self,
        context: H::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        (self.handle_function)(context)
    }

    fn destroy(
        self,
        context: H::DestroyContext,
    ) -> impl ActorFuture<'static, Result<(), ActorDestroyError>> {
        (self.destroy_function)(context)
    }
}
