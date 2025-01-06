use crate::{
    FutureRuntimeBoxFuture, FutureRuntimeHandler, FutureRuntimeReference, FutureRuntimeWaker,
};
use alloc::sync::Arc;
use futures::task::ArcWake;
use spin::Mutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FutureRuntimeTask<H>
where
    H: FutureRuntimeHandler,
{
    pub slot: Mutex<Option<FutureRuntimeBoxFuture<'static, H, ()>>>,
    pub runtime: FutureRuntimeReference<H>,
}

impl<H> ArcWake for FutureRuntimeTask<H>
where
    H: FutureRuntimeHandler,
{
    fn wake_by_ref(task: &Arc<Self>) {
        task.runtime
            .handler()
            .waker()
            .wake(task.runtime.handler(), task.clone())
    }
}

impl<H> FutureRuntimeTask<H>
where
    H: FutureRuntimeHandler,
{
    pub fn new(
        slot: Mutex<Option<FutureRuntimeBoxFuture<'static, H, ()>>>,
        runtime: FutureRuntimeReference<H>,
    ) -> Self {
        Self { slot, runtime }
    }
}
