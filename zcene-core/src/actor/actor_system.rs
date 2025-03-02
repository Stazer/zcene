use crate::actor::{
    Actor, ActorEnterError, ActorEnterable, ActorEnvironment,
    ActorSpawnError, ActorSystemCreateError, ActorSystemReference, ActorSpawnable, ActorEnvironmentAllocator
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
        S: ActorSpawnable<A, E>,
    {
        spawnable.spawn(&self.environment)
    }

    pub fn enter<S>(&self, enterable: S) -> Result<(), ActorEnterError>
    where
        S: ActorEnterable<E>,
    {
        enterable.enter(&self.environment)
    }

    pub fn enter_default(&self) -> Result<(), ActorEnterError>
    where
        (): ActorEnterable<E>,
    {
        self.enter(())
    }
}
