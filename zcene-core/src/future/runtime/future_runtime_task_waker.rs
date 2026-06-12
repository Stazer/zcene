use alloc::sync::Arc;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::task::{
    RawWaker,
    RawWakerVTable,
    Waker
};
use crate::future::runtime::{
    FutureRuntimeHandler,
    FutureRuntimeTask,
    FutureRuntimeTaskReference,
    FutureRuntimeWaker,
};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FutureRuntimeTaskWaker<H>(PhantomData<H>)
where
    H: FutureRuntimeHandler;

impl<H> FutureRuntimeTaskWaker<H>
where
    H: FutureRuntimeHandler,
{
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone,
        Self::wake,
        Self::wake_by_ref,
        Self::drop,
    );

    pub fn waker(task: FutureRuntimeTaskReference<H>) -> Waker {
        let (pointer, allocator) = Arc::into_raw_with_allocator(task);

        drop(allocator);

        unsafe { Waker::from_raw(RawWaker::new(pointer as *const (), &Self::VTABLE)) }
    }

    unsafe fn reconstruct(pointer: *const ()) -> FutureRuntimeTaskReference<H> {
        let pointer = pointer as *const FutureRuntimeTask<H>;
        let allocator = unsafe { (*pointer).runtime().handler().allocator().clone() };

        unsafe { Arc::from_raw_in(pointer, allocator) }
    }

    fn wake_task(task: &FutureRuntimeTaskReference<H>) {
        let handler = task.runtime().handler();
        handler.waker().wake(handler, task.clone());
    }

    unsafe fn clone(pointer: *const ()) -> RawWaker {
        let task = ManuallyDrop::new(unsafe { Self::reconstruct(pointer) });

        let _ = ManuallyDrop::new(FutureRuntimeTaskReference::<H>::clone(&task));

        RawWaker::new(pointer, &Self::VTABLE)
    }

    unsafe fn wake(pointer: *const ()) {
        let task = unsafe { Self::reconstruct(pointer) };
        Self::wake_task(&task);
    }

    unsafe fn wake_by_ref(pointer: *const ()) {
        let task = ManuallyDrop::new(unsafe { Self::reconstruct(pointer) });
        Self::wake_task(&task);
    }

    unsafe fn drop(pointer: *const ()) {
        drop(unsafe { Self::reconstruct(pointer) });
    }
}
