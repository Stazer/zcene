use crate::future::runtime::{
    FutureRuntimeHandler, FutureRuntimeQueue, FutureRuntimeTaskReference,
};
use concurrent_queue::{ConcurrentQueue, PushError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FutureRuntimeConcurrentQueue<H>(ConcurrentQueue<FutureRuntimeTaskReference<H>>)
where
    H: FutureRuntimeHandler;

impl<H> Default for FutureRuntimeConcurrentQueue<H>
where
    H: FutureRuntimeHandler,
{
    fn default() -> Self {
        Self(ConcurrentQueue::unbounded())
    }
}

impl<H> FutureRuntimeQueue<H> for FutureRuntimeConcurrentQueue<H>
where
    H: FutureRuntimeHandler,
{
    fn enqueue(
        &self,
        future: FutureRuntimeTaskReference<H>,
    ) -> Result<(), FutureRuntimeTaskReference<H>> {
        match self.0.push(future) {
            Ok(()) => Ok(()),
            Err(PushError::Full(future)) => {
                panic!("hello");

                Err(future)
            },
            Err(PushError::Closed(future)) => {
                panic!("hello");

                Err(future)
            },
        }
    }

    fn dequeue(&self) -> Option<FutureRuntimeTaskReference<H>> {
        self.0.pop().ok()
    }
}
