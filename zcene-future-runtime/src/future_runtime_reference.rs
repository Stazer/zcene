use crate::{FutureRuntime, FutureRuntimeHandler};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type FutureRuntimeReference<H> = Arc<FutureRuntime<H>, <H as FutureRuntimeHandler>::Allocator>;
