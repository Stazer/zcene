use crate::actor::{
    Actor, ActorAddressReference, ActorAllocatorHandler, ActorEnterError, ActorEnterHandler,
    ActorHandler, ActorSpawnError, ActorSpawnHandler, ActorSystemCreateError, ActorSystemReference,
};
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Constructor(visibility = pub(self))]
#[Method(accessors)]
pub struct ActorSystem<H>
where
    H: ActorHandler,
{
    handler: H,
}

impl<H> ActorSystem<H>
where
    H: ActorHandler,
{
    pub fn try_new(handler: H) -> Result<ActorSystemReference<H>, ActorSystemCreateError>
    where
        H: ActorAllocatorHandler,
    {
        let allocator = handler.allocator().clone();

        ActorSystemReference::try_new_in(Self::new(handler), allocator)
            .map_err(ActorSystemCreateError::from)
    }

    pub fn spawn<A>(
        &self,
        specification: H::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, H>, ActorSpawnError>
    where
        A: Actor<H>,
        H: ActorAllocatorHandler + ActorSpawnHandler,
    {
        self.handler.spawn(specification)
    }

    pub fn enter(&self, specification: H::EnterSpecification) -> Result<(), ActorEnterError>
    where
        H: ActorEnterHandler,
    {
        self.handler.enter(specification)
    }

    pub fn enter_default(&self) -> Result<(), ActorEnterError>
    where
        H: ActorEnterHandler,
        H::EnterSpecification: Default,
    {
        self.enter(H::EnterSpecification::default())
    }
}
