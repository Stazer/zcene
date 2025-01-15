use zcene_core::actor;
use zcene_core::future::runtime;
use alloc::alloc::Global;

pub struct ActorHandler {
}

impl actor::ActorHandler for ActorHandler {
    type Allocator = Global;
    type Address<A> = runtime::FutureRuntimeActorAddress<A, Self>
        where
        A: actor::Actor<Self>
        ;

    type CreateContext = ();
    type DestroyContext = ();
    type HandleContext<M> = ()
    where
        M: actor::ActorMessage;

    fn allocator(&self) -> &Self::Allocator {
        &Global
    }

    fn spawn<A>(&self, actor: A) -> Result<actor::ActorAddressReference<A, Self>, actor::ActorSpawnError>
    where
        A: actor::Actor<Self>
    {
        todo!()
    }

    fn enter(&self) -> Result<(), actor::ActorEnterError> {
        Ok(())
    }
}
