use crate::architecture::current_execution_unit_identifier;
use crate::kernel::actor::KernelActorThreadScheduler;
use crate::kernel::future::runtime::KernelFutureRuntimeHandler;
use crate::kernel::future::runtime::KernelFutureRuntimeReference;
use crate::kernel::Kernel;
use alloc::sync::Arc;
use core::marker::PhantomData;
use x86_64::instructions::interrupts::without_interrupts;
use zcene_bare::memory::address::VirtualMemoryAddress;
use zcene_bare::synchronization::Mutex;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorCommonHandleContext, ActorDiscoveryHandler, ActorEnterError,
    ActorHandler, ActorMailbox, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress,
    ActorSpawnError,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct KernelActorHandler {
    future_runtime: KernelFutureRuntimeReference,
    scheduler: Arc<Mutex<KernelActorThreadScheduler>>,
}

impl ActorHandler for KernelActorHandler {
    type Address<A>
        = ActorMessageChannelAddress<A, Self>
    where
        A: Actor<Self>;

    type Allocator = <KernelFutureRuntimeHandler as FutureRuntimeHandler>::Allocator;

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

            without_interrupts(|| {
                scheduler.lock().begin(current_execution_unit_identifier());
            });

            actor.create(()).await;

            without_interrupts(|| {
                scheduler.lock().end(current_execution_unit_identifier());
            });

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                without_interrupts(|| {
                    scheduler.lock().begin(current_execution_unit_identifier());
                });

                actor.handle(ActorCommonHandleContext::new(message)).await;

                without_interrupts(|| {
                    scheduler.lock().end(current_execution_unit_identifier());
                });
            }

            without_interrupts(|| {
                scheduler.lock().begin(current_execution_unit_identifier());
            });

            actor.destroy(()).await;

            without_interrupts(|| {
                scheduler.lock().end(current_execution_unit_identifier());
            });
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

impl KernelActorHandler {
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

impl ActorDiscoveryHandler for KernelActorHandler {
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
