use crate::actor::{
    Actor, ActorAddressReference, ActorEnterError, ActorHandler, ActorSpawnError,
    ActorSystemCreateError, ActorSystemReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

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
    pub fn try_new(handler: H) -> Result<ActorSystemReference<H>, ActorSystemCreateError> {
        let allocator = handler.allocator().clone();

        ActorSystemReference::try_new_in(Self { handler }, allocator)
            .map_err(ActorSystemCreateError::from)
    }

    pub fn spawn<A>(&self, actor: A) -> Result<ActorAddressReference<A, H>, ActorSpawnError>
    where
        A: Actor<H>,
    {
        self.handler.spawn(actor)
    }

    pub fn enter(&self) -> Result<(), ActorEnterError> {
        self.handler.enter()
    }
}
