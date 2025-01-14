use crate::future::runtime::FutureRuntimeCommonBounds;
use core::alloc::Allocator;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeAllocator = Allocator + Clone + FutureRuntimeCommonBounds;
