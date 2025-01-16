use alloc::alloc::Global;
use zcene_core::future::runtime;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct FutureRuntimeHandler {
    allocator: Global,
    queue: runtime::FutureRuntimeConcurrentQueue<Self>,
    yielder: runtime::FutureRuntimeNoOperationYielder,
    waker: runtime::FutureRuntimeContinueWaker,
}

impl runtime::FutureRuntimeHandler for FutureRuntimeHandler {
    type Allocator = Global;
    type Queue = runtime::FutureRuntimeConcurrentQueue<Self>;
    type Yielder = runtime::FutureRuntimeNoOperationYielder;
    type Waker = runtime::FutureRuntimeContinueWaker;
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

pub type FutureRuntimeReference = runtime::FutureRuntimeReference<FutureRuntimeHandler>;
