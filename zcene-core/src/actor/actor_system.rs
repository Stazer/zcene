use crate::actor::{
    Actor, ActorAllocatorHandler, ActorEnterError, ActorEnterHandler, ActorHandler,
    ActorSpawnError, ActorSpawnHandler, ActorSystemCreateError, ActorSystemReference,
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

    pub fn spawn<A, T>(
        &self,
        specification: H::SpawnSpecification<A>,
    ) -> Result<H::Address<A>, ActorSpawnError>
    where
        H: ActorSpawnHandler<T>,
        A: Actor<H> + Actor<T>,
        T: ActorHandler,
        <A as Actor<H>>::Message: From<<A as Actor<T>>::Message>,
        <A as Actor<T>>::Message: From<<A as Actor<H>>::Message>,
    {
        self.handler.spawn(specification)
    }

    pub fn spawn_inline<A>(
        &self,
        specification: H::SpawnSpecification<A>,
    ) -> Result<H::Address<A>, ActorSpawnError>
    where
        H: ActorSpawnHandler<H>,
        A: Actor<H>,
    {
        self.spawn::<A, H>(specification)
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
