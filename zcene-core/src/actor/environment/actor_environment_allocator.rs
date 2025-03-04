use crate::actor::ActorAllocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorEnvironmentAllocator {
    type Allocator: ActorAllocator;

    fn allocator(&self) -> &<Self as ActorEnvironmentAllocator>::Allocator;
}
