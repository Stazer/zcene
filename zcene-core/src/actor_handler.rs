pub use crate::{
    Actor, ActorAddress, ActorAddressReference, ActorAllocator, ActorCommonBounds, ActorEnterError,
    ActorMessage, ActorSpawnError,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorHandler: Sized + ActorCommonBounds {
    type Allocator: ActorAllocator;

    type Address<A>: ActorAddress<A, Self>
    where
        A: Actor<Self>;

    type CreateContext;
    type DestroyContext;
    type HandleContext<M>
    where
        M: ActorMessage;

    fn allocator(&self) -> &Self::Allocator;

    fn spawn<A>(&self, actor: A) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>;

    fn enter(&self) -> Result<(), ActorEnterError>;
}
