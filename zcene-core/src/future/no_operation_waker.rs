use core::ptr::null;
use core::task::{RawWaker, RawWakerVTable, Waker};

////////////////////////////////////////////////////////////////////////////////////////////////////

const NO_OPERATION_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    NoOperationWaker::raw_waker_clone,
    NoOperationWaker::no_operation,
    NoOperationWaker::no_operation,
    NoOperationWaker::no_operation,
);

pub struct NoOperationWaker;

impl NoOperationWaker {
    pub fn into_waker(self) -> Waker {
        unsafe { Waker::from_raw(Self::raw_waker()) }
    }

    const fn raw_waker() -> RawWaker {
        RawWaker::new(null(), &NO_OPERATION_WAKER_VTABLE)
    }

    unsafe fn no_operation(_data: *const ()) {}

    unsafe fn raw_waker_clone(_data: *const ()) -> RawWaker {
        Self::raw_waker()
    }
}
