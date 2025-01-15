use zcene_core::future::runtime;
use alloc::alloc::Global;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct FutureRuntimeTaskData {

}

#[derive(Default)]
pub struct FutureRuntimeHandler {
    allocator: Global,
    queue: FutureRuntimeQueue,
    yielder: runtime::FutureRuntimeNoOperationYielder,
    waker: runtime::FutureRuntimeContinueWaker,
}

impl runtime::FutureRuntimeHandler for FutureRuntimeHandler {
    type Allocator = Global;
    type Queue = FutureRuntimeQueue;
    type Yielder = runtime::FutureRuntimeNoOperationYielder;
    type Waker = runtime::FutureRuntimeContinueWaker;
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
pub struct FutureRuntimeQueue(runtime::FutureRuntimeConcurrentQueue<FutureRuntimeHandler>);

impl runtime::FutureRuntimeQueue<FutureRuntimeHandler> for FutureRuntimeQueue {
    fn enqueue(
        &self,
        future: runtime::FutureRuntimeTaskReference<FutureRuntimeHandler>,
    ) -> Result<(), runtime::FutureRuntimeTaskReference<FutureRuntimeHandler>> {
        self.0.enqueue(future)
    }

    fn dequeue(&self) -> Option<runtime::FutureRuntimeTaskReference<FutureRuntimeHandler>> {
        self.0.dequeue()
    }
}
