use crate::actor::{Actor, ActorHandler};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorContextBuilder<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn build_create_context(&self, actor: &A) -> H::CreateContext;
    fn build_handle_context(&self, actor: &A, message: &A::Message)
        -> H::HandleContext<A::Message>;
    fn build_destroy_context(&self, actor: &A) -> H::DestroyContext;
}
