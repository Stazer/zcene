use crate::actor::{ActorAllocatorHandler, ActorFuture};
use alloc::boxed::Box;
use core::pin::Pin;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorBoxFuture<'a, O, H> =
    Pin<Box<dyn ActorFuture<'a, O>, <H as ActorAllocatorHandler>::Allocator>>;
