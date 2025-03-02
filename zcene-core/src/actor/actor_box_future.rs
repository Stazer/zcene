use crate::actor::{ActorEnvironmentAllocator, ActorFuture};
use alloc::boxed::Box;
use core::pin::Pin;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorBoxFuture<'a, O, E> =
    Pin<Box<dyn ActorFuture<'a, O>, <E as ActorEnvironmentAllocator>::Allocator>>;
