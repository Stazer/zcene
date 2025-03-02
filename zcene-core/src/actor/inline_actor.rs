use crate::actor::{
    Actor, ActorCommonBounds, ActorCreateError, ActorDestroyError, ActorEnvironment, ActorFuture,
    ActorHandleError, ActorMessage,
};
use core::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct InlineActor<E, M, CF, CR, HF, HR, DF, DR>
where
    E: ActorEnvironment,
    M: ActorMessage,
    CF: FnMut(E::CreateContext) -> CR + ActorCommonBounds,
    CR: for<'a> ActorFuture<'a, Result<(), ActorCreateError>> + ActorCommonBounds,
    HF: FnMut(E::HandleContext<M>) -> HR + ActorCommonBounds,
    HR: for<'a> ActorFuture<'a, Result<(), ActorHandleError>> + ActorCommonBounds,
    DF: FnOnce(E::DestroyContext) -> DR + ActorCommonBounds,
    DR: for<'a> ActorFuture<'a, Result<(), ActorDestroyError>> + ActorCommonBounds,
{
    create_function: CF,
    handle_function: HF,
    destroy_function: DF,
    types: PhantomData<(E, M, CR, HR, DR)>,
}

impl<E, M, CF, CR, HF, HR, DF, DR> InlineActor<E, M, CF, CR, HF, HR, DF, DR>
where
    E: ActorEnvironment,
    M: ActorMessage,
    CF: FnMut(E::CreateContext) -> CR + ActorCommonBounds,
    CR: for<'a> ActorFuture<'a, Result<(), ActorCreateError>> + ActorCommonBounds,
    HF: FnMut(E::HandleContext<M>) -> HR + ActorCommonBounds,
    HR: for<'a> ActorFuture<'a, Result<(), ActorHandleError>> + ActorCommonBounds,
    DF: FnOnce(E::DestroyContext) -> DR + ActorCommonBounds,
    DR: for<'a> ActorFuture<'a, Result<(), ActorDestroyError>> + ActorCommonBounds,
{
    pub fn new(create_function: CF, handle_function: HF, destroy_function: DF) -> Self {
        Self {
            create_function,
            handle_function,
            destroy_function,
            types: PhantomData::<(E, M, CR, HR, DR)>,
        }
    }
}

impl<E, M, CF, CR, HF, HR, DF, DR> Actor<E> for InlineActor<E, M, CF, CR, HF, HR, DF, DR>
where
    E: ActorEnvironment,
    M: ActorMessage,
    CF: FnMut(E::CreateContext) -> CR + ActorCommonBounds,
    CR: for<'a> ActorFuture<'a, Result<(), ActorCreateError>> + ActorCommonBounds,
    HF: FnMut(E::HandleContext<M>) -> HR + ActorCommonBounds,
    HR: for<'a> ActorFuture<'a, Result<(), ActorHandleError>> + ActorCommonBounds,
    DF: FnOnce(E::DestroyContext) -> DR + ActorCommonBounds,
    DR: for<'a> ActorFuture<'a, Result<(), ActorDestroyError>> + ActorCommonBounds,
{
    type Message = M;

    fn create(
        &mut self,
        context: E::CreateContext,
    ) -> impl ActorFuture<'_, Result<(), ActorCreateError>> {
        (self.create_function)(context)
    }

    fn handle(
        &mut self,
        context: E::HandleContext<Self::Message>,
    ) -> impl ActorFuture<'_, Result<(), ActorHandleError>> {
        (self.handle_function)(context)
    }

    fn destroy(
        self,
        context: E::DestroyContext,
    ) -> impl ActorFuture<'static, Result<(), ActorDestroyError>> {
        (self.destroy_function)(context)
    }
}
