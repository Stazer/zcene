use crate::actor::ActorAllocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAllocatorHandler {
    type Allocator: ActorAllocator;

    fn allocator(&self) -> &<Self as ActorAllocatorHandler>::Allocator;
}
