use alloc::alloc::Global;
use zcene_core::future::runtime::{
    self as core,
    FutureRuntimeConcurrentQueue, FutureRuntimeContinueWaker,
    FutureRuntimeNoOperationYielder,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct FutureRuntimeHandler {
    allocator: Global,
    queue: FutureRuntimeConcurrentQueue<Self>,
    yielder: FutureRuntimeNoOperationYielder,
    waker: FutureRuntimeContinueWaker,
}

impl core::FutureRuntimeHandler for FutureRuntimeHandler {
    type Allocator = Global;
    type Queue = FutureRuntimeConcurrentQueue<Self>;
    type Yielder = FutureRuntimeNoOperationYielder;
    type Waker = FutureRuntimeContinueWaker;

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
