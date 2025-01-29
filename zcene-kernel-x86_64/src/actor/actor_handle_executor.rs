use crate::actor::{ActorHandler, ActorThreadScheduler};
use crate::architecture::current_execution_unit_identifier;
use alloc::sync::Arc;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};
use pin_project::pin_project;
use x86_64::instructions::interrupts::without_interrupts;
use zcene_core::actor::{Actor, ActorCommonHandleContext, ActorHandleError};
use zcene_core::future::runtime::FutureRuntimeHandler;
use zcene_kernel::synchronization::Mutex;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
#[pin_project]
pub struct ActorHandleExecutor<'a, A, H>
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    actor: &'a mut A,
    message: A::Message,
    scheduler: Arc<Mutex<ActorThreadScheduler>>,
    handler: PhantomData<H>,
}

impl<'a, A, H> Future for ActorHandleExecutor<'a, A, H>
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    type Output = Result<(), ActorHandleError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let scheduler = self.scheduler.clone();
        let message = self.message.clone();

        let mut pinned = pin!(self.actor.handle(ActorCommonHandleContext::new(message)));

        without_interrupts(|| {
            scheduler.lock().begin(current_execution_unit_identifier());
        });

        let result = pinned.as_mut().poll(context);

        without_interrupts(|| {
            scheduler.lock().end(current_execution_unit_identifier());
        });

        result
    }
}
