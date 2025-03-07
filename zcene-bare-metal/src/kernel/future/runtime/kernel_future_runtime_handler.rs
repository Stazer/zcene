use alloc::alloc::Global;
use zcene_core::future::runtime::{
    FutureRuntimeConcurrentQueue, FutureRuntimeContinueWaker, FutureRuntimeHandler,
    FutureRuntimeNoOperationYielder,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct KernelFutureRuntimeHandler {
    allocator: Global,
    queue: FutureRuntimeConcurrentQueue<Self>,
    yielder: FutureRuntimeNoOperationYielder,
    waker: FutureRuntimeContinueWaker,
}

impl FutureRuntimeHandler for KernelFutureRuntimeHandler {
    type Allocator = Global;
    type Queue = FutureRuntimeConcurrentQueue<Self>;
    type Yielder = FutureRuntimeNoOperationYielder;
    type Waker = FutureRuntimeContinueWaker;
    type Specification = ();
    type Data = ();

    fn allocator(&self) -> &Self::Allocator {
        &self.allocator
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
