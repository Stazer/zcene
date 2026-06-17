use crate::future::runtime::{
    FutureRuntimeHandler, FutureRuntimeQueue, FutureRuntimeTaskReference,
};
use crate::queue::{ConcurrentQueue, PushError};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FutureRuntimeConcurrentQueue<H>(
    ConcurrentQueue<FutureRuntimeTaskReference<H>, <H as FutureRuntimeHandler>::Allocator>,
)
where
    H: FutureRuntimeHandler;

impl<H> FutureRuntimeConcurrentQueue<H>
where
    H: FutureRuntimeHandler,
{
    pub fn new_in(allocator: <H as FutureRuntimeHandler>::Allocator) -> Self {
        Self(ConcurrentQueue::unbounded_in(allocator))
    }
}

impl<H> Default for FutureRuntimeConcurrentQueue<H>
where
    H: FutureRuntimeHandler,
    <H as FutureRuntimeHandler>::Allocator: Default,
{
    fn default() -> Self {
        Self::new_in(<H as FutureRuntimeHandler>::Allocator::default())
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
            Err(PushError::Full(future)) => Err(future),
            Err(PushError::Closed(future)) => Err(future),
        }
    }

    fn dequeue(&self) -> Option<FutureRuntimeTaskReference<H>> {
        self.0.pop().ok()
    }
}
