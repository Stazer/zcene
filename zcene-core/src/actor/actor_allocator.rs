use crate::actor::ActorCommonBounds;
use core::alloc::Allocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorAllocator = Allocator + Clone + ActorCommonBounds;
