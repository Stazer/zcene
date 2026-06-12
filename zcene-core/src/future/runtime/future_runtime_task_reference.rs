use crate::future::runtime::{FutureRuntimeHandler, FutureRuntimeTask};
use alloc::sync::Arc;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type FutureRuntimeTaskReference<H> =
    Arc<FutureRuntimeTask<H>, <H as FutureRuntimeHandler>::Allocator>;
