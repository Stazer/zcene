use crate::actor::{Actor, ActorCommonHandleContext, ActorContextBuilder, ActorHandler};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct ActorCommonContextBuilder;

impl<A, H> ActorContextBuilder<A, H> for ActorCommonContextBuilder
where
    A: Actor<H>,
    H: ActorHandler<
        HandleContext<A::Message> = ActorCommonHandleContext<A::Message>,
        CreateContext = (),
        DestroyContext = (),
    >,
{
    fn build_create_context(&self, _actor: &A) -> H::CreateContext {}

    fn build_handle_context(
        &self,
        _actor: &A,
        message: &A::Message,
    ) -> H::HandleContext<A::Message> {
        ActorCommonHandleContext::new(message.clone())
    }

    fn build_destroy_context(&self, _actor: &A) -> H::DestroyContext {}
}
