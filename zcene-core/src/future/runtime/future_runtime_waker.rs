use crate::future::runtime::{
    FutureRuntimeCommonBounds, FutureRuntimeHandler, FutureRuntimeTaskReference,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeWaker<H>: FutureRuntimeCommonBounds
where
    H: FutureRuntimeHandler,
{
    fn wake(&self, handler: &H, task: FutureRuntimeTaskReference<H>);
}
