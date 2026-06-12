use crate::common::memory::allocator::ErasedAllocator;
use zcene_core::future::runtime::{
    self as core, FutureRuntimeConcurrentQueue, FutureRuntimeContinueWaker,
    FutureRuntimeNoOperationYielder,
};
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FutureRuntimeHandler {
    allocator: ErasedAllocator,
    queue: FutureRuntimeConcurrentQueue<Self>,
    yielder: FutureRuntimeNoOperationYielder,
    waker: FutureRuntimeContinueWaker,
}

impl FutureRuntimeHandler {
    pub fn new(allocator: ErasedAllocator) -> Self {
        Self {
            queue: FutureRuntimeConcurrentQueue::new_in(allocator.clone()),
            allocator,
            yielder: FutureRuntimeNoOperationYielder::default(),
            waker: FutureRuntimeContinueWaker::default(),
        }
    }
}

impl core::FutureRuntimeHandler for FutureRuntimeHandler {
    type Allocator = ErasedAllocator;
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
