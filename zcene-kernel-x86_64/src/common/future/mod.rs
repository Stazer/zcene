use zcene_core::future as future;
use alloc::alloc::Global;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct FutureRuntimeTaskData {

}

#[derive(Default)]
pub struct FutureRuntimeHandler {
    allocator: Global,
    queue: FutureRuntimeQueue,
    yielder: future::FutureRuntimeNoOperationYielder,
    waker: future::FutureRuntimeContinueWaker,
}

impl future::FutureRuntimeHandler for FutureRuntimeHandler {
    type Allocator = Global;
    type Queue = FutureRuntimeQueue;
    type Yielder = future::FutureRuntimeNoOperationYielder;
    type Waker = future::FutureRuntimeContinueWaker;
    type Data = FutureRuntimeTaskData;

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

#[derive(Default)]
pub struct FutureRuntimeQueue(future::FutureRuntimeConcurrentQueue<FutureRuntimeHandler>);

impl future::FutureRuntimeQueue<FutureRuntimeHandler> for FutureRuntimeQueue {
    fn enqueue(
        &self,
        future: future::FutureRuntimeTaskReference<FutureRuntimeHandler>,
    ) -> Result<(), future::FutureRuntimeTaskReference<FutureRuntimeHandler>> {
        self.0.enqueue(future)
    }

    fn dequeue(&self) -> Option<future::FutureRuntimeTaskReference<FutureRuntimeHandler>> {
        self.0.dequeue()
    }
}
