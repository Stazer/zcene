use crate::{FutureRuntimeHandler, FutureRuntimeTaskReference};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeQueue<H>: Send + Sync + 'static
where
    H: FutureRuntimeHandler,
{
    fn enqueue(
        &self,
        future: FutureRuntimeTaskReference<H>,
    ) -> Result<(), FutureRuntimeTaskReference<H>>;
    fn dequeue(&self) -> Option<FutureRuntimeTaskReference<H>>;
}
