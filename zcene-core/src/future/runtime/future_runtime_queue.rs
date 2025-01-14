use crate::future::runtime::{
    FutureRuntimeCommonBounds, FutureRuntimeHandler, FutureRuntimeTaskReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeQueue<H>: FutureRuntimeCommonBounds
where
    H: FutureRuntimeHandler,
{
    fn enqueue(
        &self,
        future: FutureRuntimeTaskReference<H>,
    ) -> Result<(), FutureRuntimeTaskReference<H>>;
    fn dequeue(&self) -> Option<FutureRuntimeTaskReference<H>>;
}
