use crate::actor::{
    Actor, ActorAllocatorHandler, ActorEnterError, ActorEnterHandler, ActorEnvironment,
    ActorSpawnError, ActorSystemCreateError, ActorSystemReference, ActorSpawnable, ActorSpawner
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
        E: ActorAllocatorHandler,
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
        E: ActorSpawner<A, E>,
        S: ActorSpawnable<A, E>,
    {
        self.environment.spawn(spawnable)
    }

    pub fn enter(&self, specification: E::EnterSpecification) -> Result<(), ActorEnterError>
    where
        E: ActorEnterHandler,
    {
        self.environment.enter(specification)
    }

    pub fn enter_default(&self) -> Result<(), ActorEnterError>
    where
        E: ActorEnterHandler,
        E::EnterSpecification: Default,
    {
        self.enter(E::EnterSpecification::default())
    }
}
