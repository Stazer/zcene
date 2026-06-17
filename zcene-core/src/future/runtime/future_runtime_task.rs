use crate::future::runtime::{FutureRuntimeBoxFuture, FutureRuntimeHandler, FutureRuntimeReference};
use spin::Mutex;
use ztd::{Constructor, Method};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(all)]
pub struct FutureRuntimeTask<H>
where
    H: FutureRuntimeHandler,
{
    // FIXME: Remove mutex
    slot: Mutex<Option<FutureRuntimeBoxFuture<'static, H, ()>>>,
    runtime: FutureRuntimeReference<H>,
}
