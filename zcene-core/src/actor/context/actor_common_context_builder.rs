use crate::actor::{Actor, ActorCommonHandleContext, ActorContextBuilder, ActorEnvironment};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct ActorCommonContextBuilder;

impl<A, E> ActorContextBuilder<A, E> for ActorCommonContextBuilder
where
    A: Actor<E>,
    E: ActorEnvironment<
        HandleContext<A::Message> = ActorCommonHandleContext<A::Message>,
        CreateContext = (),
        DestroyContext = (),
    >,
{
    fn build_create_context(&self, _actor: &A) -> E::CreateContext {}

    fn build_handle_context(
        &self,
        _actor: &A,
        message: &A::Message,
    ) -> E::HandleContext<A::Message> {
        ActorCommonHandleContext::new(message.clone())
    }

    fn build_destroy_context(&self, _actor: &A) -> E::DestroyContext {}
}
