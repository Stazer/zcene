use crate::future::runtime::{
    FutureRuntimeBoxFuture, FutureRuntimeHandler, FutureRuntimeReference, FutureRuntimeWaker,
};
use alloc::sync::Arc;
use futures::task::ArcWake;
use spin::Mutex;
use ztd::{Method, Constructor};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Method)]
#[Method(all)]
pub struct FutureRuntimeTask<H>
where
    H: FutureRuntimeHandler,
{
    slot: Mutex<Option<FutureRuntimeBoxFuture<'static, H, ()>>>,
    runtime: FutureRuntimeReference<H>,
    data: H::Data,
}

impl<H> ArcWake for FutureRuntimeTask<H>
where
    H: FutureRuntimeHandler,
{
    fn wake_by_ref(task: &Arc<Self>) {
        task.runtime
            .handler()
            .waker()
            .wake(task.runtime().handler(), task.clone())
    }
}
