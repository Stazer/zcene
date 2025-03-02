use crate::actor::{
    Actor, ActorEnterError, ActorEnvironmentEnterable, ActorEnvironment,
    ActorSpawnError, ActorSystemCreateError, ActorSystemReference, ActorEnvironmentSpawnable, ActorEnvironmentAllocator
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

    pub fn spawn<A, S>(
        &self,
        spawnable: S
    ) -> Result<E::Address<A>, ActorSpawnError>
    where
        A: Actor<E>,
        S: ActorEnvironmentSpawnable<A, E>,
    {
        spawnable.spawn(&self.environment)
    }

    pub fn enter_with<S>(&self, enterable: S) -> Result<(), ActorEnterError>
    where
        S: ActorEnvironmentEnterable<E>,
    {
        enterable.enter(&self.environment)
    }

    pub fn enter_default<S>(&self) -> Result<(), ActorEnterError>
    where
        S: ActorEnvironmentEnterable<E> + Default,
    {
        S::default().enter(&self.environment)
    }

    pub fn enter(&self) -> Result<(), ActorEnterError>
    where
        (): ActorEnvironmentEnterable<E>,
    {
        self.enter_with(())
    }
}
