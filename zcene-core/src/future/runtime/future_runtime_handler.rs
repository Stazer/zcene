use crate::future::runtime::{
    FutureRuntimeAllocator, FutureRuntimeCommonBounds, FutureRuntimeQueue, FutureRuntimeWaker,
    FutureRuntimeYielder
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureRuntimeHandler: FutureRuntimeCommonBounds
where
    Self: Sized,
{
    type Allocator: FutureRuntimeAllocator;
    type Queue: FutureRuntimeQueue<Self>;
    type Yielder: FutureRuntimeYielder;
    type Waker: FutureRuntimeWaker<Self>;
    type Data: Default + FutureRuntimeCommonBounds;

    fn allocator(&self) -> &Self::Allocator;
    fn queue(&self) -> &Self::Queue;
    fn yielder(&self) -> &Self::Yielder;
    fn waker(&self) -> &Self::Waker;
}
