use crate::actor::ActorCommonBounds;
use core::alloc::{Allocator, AllocatorClone, StaticAllocator};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAllocator = Allocator + AllocatorClone + StaticAllocator + Clone + ActorCommonBounds;
