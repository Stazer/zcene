use crate::actor::{Actor, ActorEnvironment};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorContextBuilder<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment,
{
    fn build_create_context(&self, actor: &A) -> E::CreateContext;
    fn build_handle_context(&self, actor: &A, message: &A::Message)
        -> E::HandleContext<A::Message>;
    fn build_destroy_context(&self, actor: &A) -> E::DestroyContext;
}
