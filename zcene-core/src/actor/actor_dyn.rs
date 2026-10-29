use crate::actor::{
    ActorBoxFuture, ActorCreateError, ActorDestroyError, ActorEnvironment,
    ActorEnvironmentAllocator, ActorHandleError, ActorMessage,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Object-safe view of [`Actor`](crate::actor::Actor), so an executor can hold an actor
/// behind a `Box<dyn ActorDyn<E, Message = M>, _>` and swap the concrete implementation at
/// runtime (actor migration) without changing the message type flowing through the recipient.
pub trait ActorDyn<E>: Send + Sync + 'static
where
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    type Message: ActorMessage;

    fn create<'a>(
        &'a mut self,
        context: E::CreateContext<'a>,
    ) -> ActorBoxFuture<'a, Result<(), ActorCreateError>, E>;

    fn handle<'a>(
        &'a mut self,
        context: E::HandleContext<'a, Self::Message>,
    ) -> ActorBoxFuture<'a, Result<(), ActorHandleError>, E>;

    fn destroy<'a>(
        &'a mut self,
        context: E::DestroyContext<'a>,
    ) -> ActorBoxFuture<'a, Result<(), ActorDestroyError>, E>;
}
