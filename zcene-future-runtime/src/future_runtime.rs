use crate::{
    FutureRuntimeCreateError, FutureRuntimeFuture, FutureRuntimeHandler, FutureRuntimeQueue,
    FutureRuntimeReference, FutureRuntimeSpawnError, FutureRuntimeTask, FutureRuntimeYielder,
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::task::Context;
use futures::task::waker_ref;
use spin::Mutex;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FutureRuntime<H>
where
    H: FutureRuntimeHandler,
{
    handler: H,
}

impl<H> FutureRuntime<H>
where
    H: FutureRuntimeHandler,
{
    pub fn new(handler: H) -> Result<FutureRuntimeReference<H>, FutureRuntimeCreateError> {
        let allocator = handler.allocator().clone();

        FutureRuntimeReference::try_new_in(Self { handler }, allocator)
            .map_err(FutureRuntimeCreateError::from)
    }

    pub fn spawn<F>(
        self: &FutureRuntimeReference<H>,
        future: F,
    ) -> Result<(), FutureRuntimeSpawnError>
    where
        F: FutureRuntimeFuture<'static, ()>,
    {
        let allocator = self.handler.allocator().clone();

        self.handler
            .queue()
            .enqueue(Arc::new(FutureRuntimeTask {
                slot: Mutex::new(Some(Box::pin_in(future, allocator))),
                runtime: self.clone(),
            }))
            .map_err(|_| FutureRuntimeSpawnError::Busy)
    }

    pub fn run(self: &FutureRuntimeReference<H>) {
        loop {
            let task = match self.handler.queue().dequeue() {
                Some(task) => task,
                None => {
                    self.handler.yielder().r#yield();
                    continue;
                }
            };

            let mut future_slot = task.slot.lock();

            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);

                if future.as_mut().poll(context).is_pending() {
                    *future_slot = Some(future);
                }
            }
        }
    }

    pub(crate) fn handler(&self) -> &H {
        &self.handler
    }
}
