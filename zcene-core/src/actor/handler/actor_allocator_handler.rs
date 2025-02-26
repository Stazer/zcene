use crate::actor::{ActorAllocator, ActorHandler};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAllocatorHandler: ActorHandler {
    type Allocator: ActorAllocator;

    fn allocator(&self) -> &<Self as ActorAllocatorHandler>::Allocator;
}
