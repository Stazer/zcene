use crate::future::runtime::{
    FutureRuntimeHandler, FutureRuntimeQueue, FutureRuntimeTaskReference, FutureRuntimeWaker,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct FutureRuntimeContinueWaker;

impl<H> FutureRuntimeWaker<H> for FutureRuntimeContinueWaker
where
    H: FutureRuntimeHandler,
{
    fn wake(&self, handler: &H, task: FutureRuntimeTaskReference<H>) {
        while handler.queue().enqueue(task.clone()).is_err() {
            panic!("HELLO!");
            // noop
        }
    }
}
