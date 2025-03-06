use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub async fn r#yield() {
    #[derive(Default)]
    struct Yield {
        yielded: bool,
    }

    impl Future for Yield {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
            if self.yielded {
                return Poll::Ready(());
            }

            self.yielded = true;

            context.waker().wake_by_ref();

            Poll::Pending
        }
    }

    Yield::default().await;
}
