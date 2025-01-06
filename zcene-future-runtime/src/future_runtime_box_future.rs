use crate::{FutureRuntimeFuture, FutureRuntimeHandler};
use alloc::boxed::Box;
use core::pin::Pin;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type FutureRuntimeBoxFuture<'a, H, O> =
    Pin<Box<dyn FutureRuntimeFuture<'a, O>, <H as FutureRuntimeHandler>::Allocator>>;
