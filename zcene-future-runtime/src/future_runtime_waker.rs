use crate::{FutureRuntimeHandler, FutureRuntimeTaskReference};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeWaker<H>: Send + Sync + 'static
where
    H: FutureRuntimeHandler,
{
    fn wake(&self, handler: &H, task: FutureRuntimeTaskReference<H>);
}
