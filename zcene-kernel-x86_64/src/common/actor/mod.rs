use core::marker::PhantomData;
use zcene_core::actor;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorEnterError, ActorMessage, ActorMessageChannel,
    ActorMessageChannelAddress, ActorSpawnError,
};
use zcene_core::future::runtime::{
    FutureRuntimeActorHandleContext, FutureRuntimeHandler, FutureRuntimeReference,
};
use ztd::Constructor;
use crate::kernel::Kernel;
use core::fmt::Write;


////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorIdentifier = usize;

use core::sync::atomic::AtomicUsize;
use alloc::collections::{BTreeSet, BTreeMap, VecDeque};
use alloc::vec::Vec;

#[derive(Default)]
pub struct Handle {
    identifier: ActorIdentifier,
    stack_frame: spin::Mutex<Option<InterruptStackFrameValue>>,
}

impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.identifier.eq(&other.identifier)
    }
}

impl Eq for Handle {
}

impl PartialOrd for Handle {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.identifier.partial_cmp(&other.identifier)
    }
}

impl Ord for Handle {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.identifier.cmp(&other.identifier)
    }
}

#[derive(Default)]
pub struct Shared
{
    all: spin::Mutex<BTreeSet<Arc<Handle>>>,
    next: spin::Mutex<Vec<Arc<Handle>>>,
    threads: spin::Mutex<BTreeMap<usize, Arc<Handle>>>,
    identifier_counter: AtomicUsize,
}

use alloc::sync::Arc;

#[derive(Constructor)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
    shared: Arc<Shared>
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
        = FutureRuntimeActorHandleContext<M>
    where
        M: ActorMessage;
    type DestroyContext = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(&self, mut actor: A) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let identifier = self.shared.identifier_counter.fetch_add(
            1,
            core::sync::atomic::Ordering::SeqCst,
        );

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            Self::Address::new(sender, PhantomData),
            self.allocator().clone(),
        )?;

        let shared = self.shared.clone();

        self.future_runtime.spawn(async move {
            let handle = Arc::new(Handle {
                identifier,
                stack_frame: spin::Mutex::default(),
            });

            shared.all.lock().insert(handle.clone());

            actor.create(()).await;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => break,
                };

                {
                    let mut threads = shared.threads.lock();
                    threads.insert(
                        crate::common::x86::initial_local_apic_id().unwrap(),
                        handle.clone(),
                    );
                }

                actor
                    .handle(Self::HandleContext::<A::Message>::new(message))
                    .await;

                {
                    let mut threads = shared.threads.lock();
                    threads.remove(
                        &crate::common::x86::initial_local_apic_id().unwrap(),
                    );
                }

            }

            actor.destroy(()).await;

            shared.all.lock().remove(&handle);
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

use x86_64::structures::idt::{InterruptStackFrameValue};

impl<H> ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    pub fn reschedule(&self, old_stack_frame: InterruptStackFrameValue) -> Option<InterruptStackFrameValue> {
        let id = crate::common::x86::initial_local_apic_id()?;

        let mut threads = self.shared.threads.lock();

        let from_handle = threads.get_mut(&id)?;
        *from_handle.stack_frame.lock() = Some(old_stack_frame);

        let mut next = self.shared.next.lock();
        if next.is_empty() {
            *next = self.shared.all.lock().iter().cloned().collect();
        }

        let next_handle = next.pop()?;

        //Kernel::get().logger().writer(|w| write!(w, "next {}\n", next_handle.identifier));

        let new_stack_frame = next_handle.stack_frame.lock().take();

        match new_stack_frame {
            Some(new_stack_frame) => {
                Kernel::get().logger().writer(|w| write!(w, "restore {}\n", next_handle.identifier));

                Some(new_stack_frame)
            },
            None => {
                Some(
                    InterruptStackFrameValue::new(
                        x86_64::addr::VirtAddr::new((hello as usize).try_into().unwrap()),
                        old_stack_frame.code_segment,
                        old_stack_frame.cpu_flags,
                        old_stack_frame.stack_pointer,
                        old_stack_frame.stack_segment,
                    )
                )
            }
        }
    }
}

fn hello() -> ! {
    use x86::apic::x2apic::X2APIC;
    use x86::apic::ApicControl;

    //Kernel::get().logger().writer(|w| write!(w, "hello2\n"));
    Kernel::get().actor_system().enter();

    loop {}
}
