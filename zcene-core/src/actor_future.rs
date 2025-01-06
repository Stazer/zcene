use core::future::Future;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorFuture<'a, O> = Future<Output = O> + Send + 'a;
