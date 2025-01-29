use crate::actor::{ActorHandleExecutor, ActorThreadScheduler};
use crate::architecture::current_execution_unit_identifier;
use crate::kernel::Kernel;
use alloc::sync::Arc;
use core::marker::PhantomData;
use x86_64::instructions::interrupts::without_interrupts;
use zcene_core::actor;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorCommonHandleContext, ActorDiscoveryHandler, ActorEnterError,
    ActorMailbox, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress, ActorSpawnError,
};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use zcene_kernel::memory::address::VirtualMemoryAddress;
use zcene_kernel::synchronization::Mutex;
use ztd::Constructor;
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
    scheduler: Arc<Mutex<ActorThreadScheduler>>,
    //mailboxes: Mutex<HashMap<Any, ActorWeakMailbox>>,
}

impl<H> actor::ActorHandler for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type Allocator = H::Allocator;

    type CreateContext = ();
    type HandleContext<M>
        = ActorCommonHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(&self, actor: A) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        let scheduler = self.scheduler.clone();

        self.future_runtime.spawn(async move {
            let mut actor = actor;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                ActorHandleExecutor::new(&mut actor, message, scheduler.clone(), PhantomData::<H>)
                    .await;
            }
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

impl<H> ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    pub fn reschedule(&self, stack_pointer: VirtualMemoryAddress) -> VirtualMemoryAddress {
        let mut scheduler = self.scheduler.lock();

        let next_thread = scheduler.queue_mut().pop_front();

        scheduler.r#break(
            current_execution_unit_identifier(),
            VirtualMemoryAddress::from(stack_pointer),
        );

        scheduler.r#continue(current_execution_unit_identifier(), next_thread)
    }
}

impl<H> ActorDiscoveryHandler for ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    fn discover<M>(&self) -> Option<ActorMailbox<M, Self>>
    where
        M: ActorMessage,
    {
        None
    }
}

pub fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}
