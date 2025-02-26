use crate::actor::{ActorFuture, ActorAllocatorHandler};
use alloc::boxed::Box;
use core::pin::Pin;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorBoxFuture<'a, O, H> =
    Pin<Box<dyn ActorFuture<'a, O>, <H as ActorAllocatorHandler>::Allocator>>;
