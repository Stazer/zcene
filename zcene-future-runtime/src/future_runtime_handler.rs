use crate::{FutureRuntimeAllocator, FutureRuntimeQueue, FutureRuntimeWaker, FutureRuntimeYielder};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeHandler: Send + Sync + 'static
where
    Self: Sized,
{
    type Allocator: FutureRuntimeAllocator;
    type Queue: FutureRuntimeQueue<Self>;
    type Yielder: FutureRuntimeYielder;
    type Waker: FutureRuntimeWaker<Self>;

    fn allocator(&self) -> &Self::Allocator;
    fn queue(&self) -> &Self::Queue;
    fn yielder(&self) -> &Self::Yielder;
    fn waker(&self) -> &Self::Waker;
}
