use crate::future::runtime::FutureRuntimeCommonBounds;
use core::alloc::{Allocator, AllocatorClone, StaticAllocator};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeAllocator =
    Allocator + AllocatorClone + StaticAllocator + Clone + FutureRuntimeCommonBounds;
