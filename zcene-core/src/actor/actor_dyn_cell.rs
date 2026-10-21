use crate::actor::{
    Actor, ActorBoxFuture, ActorCreateError, ActorDestroyError, ActorDyn, ActorEnvironment,
    ActorEnvironmentAllocator, ActorHandleError,
};
use alloc::boxed::Box;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// The concrete [`ActorDyn`] implementation: owns one real `A: Actor<E>` and erases it
/// behind the object-safe interface so it can be boxed as `Box<dyn ActorDyn<E, A::Message>, _>`.
pub struct ActorDynCell<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    actor: A,
    allocator: E::Allocator,
}

impl<A, E> ActorDynCell<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    pub fn new(actor: A, allocator: E::Allocator) -> Self {
        Self { actor, allocator }
    }
}

impl<A, E> ActorDyn<E> for ActorDynCell<A, E>
where
    A: Actor<E>,
    E: ActorEnvironment + ActorEnvironmentAllocator,
{
    type Message = A::Message;

    fn create<'a>(
        &'a mut self,
        context: E::CreateContext<'a>,
    ) -> ActorBoxFuture<'a, Result<(), ActorCreateError>, E> {
        let allocator = self.allocator.clone();

        Box::pin_in(self.actor.create(context), allocator)
    }

    fn handle<'a>(
        &'a mut self,
        context: E::HandleContext<'a, A::Message>,
    ) -> ActorBoxFuture<'a, Result<(), ActorHandleError>, E> {
        let allocator = self.allocator.clone();

        Box::pin_in(self.actor.handle(context), allocator)
    }

    fn destroy<'a>(
        &'a mut self,
        context: E::DestroyContext<'a>,
    ) -> ActorBoxFuture<'a, Result<(), ActorDestroyError>, E> {
        let allocator = self.allocator.clone();

        Box::pin_in(self.actor.destroy(context), allocator)
    }
}
