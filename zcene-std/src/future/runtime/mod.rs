use std::alloc::Global;
use zcene_core::future::runtime::{
    self,
    FutureRuntimeConcurrentQueue, FutureRuntimeContinueWaker,
    FutureRuntimeNoOperationYielder,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct FutureRuntimeHandler {
    queue: FutureRuntimeConcurrentQueue<Self>,
    yielder: FutureRuntimeNoOperationYielder,
    waker: FutureRuntimeContinueWaker,
}

impl runtime::FutureRuntimeHandler for FutureRuntimeHandler {
    type Allocator = Global;
    type Queue = FutureRuntimeConcurrentQueue<Self>;
    type Yielder = FutureRuntimeNoOperationYielder;
    type Waker = FutureRuntimeContinueWaker;
    type Specification = ();
    type Data = ();

    fn allocator(&self) -> &Self::Allocator {
        &Global
    }

    fn queue(&self) -> &Self::Queue {
        &self.queue
    }

    fn yielder(&self) -> &Self::Yielder {
        &self.yielder
    }

    fn waker(&self) -> &Self::Waker {
        &self.waker
    }
}
