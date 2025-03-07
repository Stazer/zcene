use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait FutureExt<O> {
    fn complete(self) -> O;
}

impl<T, O> FutureExt<O> for T
where
    T: Future<Output = O>,
{
    fn complete(self) -> O {
        let mut pinned_future = pin!(self);
        let waker = Waker::noop();
        let mut context = Context::from_waker(&waker);

        loop {
            if let Poll::Ready(output) = pinned_future.as_mut().poll(&mut context) {
                break output;
            }
        }
    }
}
