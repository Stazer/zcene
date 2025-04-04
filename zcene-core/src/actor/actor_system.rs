use crate::actor::{
    Actor, ActorEnterError, ActorEnvironment, ActorEnvironmentAllocator, ActorEnvironmentEnterable,
    ActorEnvironmentSpawnable, ActorSpawnError, ActorSystemCreateError, ActorSystemReference,
};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Constructor(visibility = pub(self))]
#[Method(accessors)]
pub struct ActorSystem<E>
where
    E: ActorEnvironment,
{
    environment: E,
}

impl<E> ActorSystem<E>
where
    E: ActorEnvironment,
{
    pub fn try_new(environment: E) -> Result<ActorSystemReference<E>, ActorSystemCreateError>
    where
        E: ActorEnvironmentAllocator,
    {
        let allocator = environment.allocator().clone();

        ActorSystemReference::try_new_in(Self::new(environment), allocator)
            .map_err(ActorSystemCreateError::from)
    }

    pub fn allocator(&self) -> &E::Allocator
    where
        E: ActorEnvironmentAllocator,
    {
        self.environment.allocator()
    }

    pub fn spawn<A, S>(
        self: &ActorSystemReference<E>,
        spawnable: S,
    ) -> Result<S::Address, ActorSpawnError>
    where
        A: Actor<E>,
        E: ActorEnvironmentAllocator,
        S: ActorEnvironmentSpawnable<A, E>,
    {
        spawnable.spawn(self)
    }

    pub fn enter_with<S>(
        self: &ActorSystemReference<E>,
        enterable: S,
    ) -> Result<(), ActorEnterError>
    where
        E: ActorEnvironmentAllocator,
        S: ActorEnvironmentEnterable<E>,
    {
        enterable.enter(self)
    }

    pub fn enter_default<S>(self: &ActorSystemReference<E>) -> Result<(), ActorEnterError>
    where
        E: ActorEnvironmentAllocator,
        S: ActorEnvironmentEnterable<E> + Default,
    {
        S::default().enter(self)
    }

    pub fn enter(self: &ActorSystemReference<E>) -> Result<(), ActorEnterError>
    where
        E: ActorEnvironmentAllocator,
        (): ActorEnvironmentEnterable<E>,
    {
        self.enter_with(())
    }
}
