use crate::{ActorFuture, ActorHandler};
use alloc::boxed::Box;
use core::pin::Pin;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorBoxFuture<'a, O, H> =
    Pin<Box<dyn ActorFuture<'a, O>, <H as ActorHandler>::Allocator>>;
