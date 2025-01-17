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

struct Context {
    sp: u64,
    ip: u64,
}

#[derive(Default)]
pub struct Handle {
    identifier: ActorIdentifier,
    context: spin::Mutex<Option<Context>>,
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
                context: spin::Mutex::default(),
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
    pub fn reschedule(&self, ip: u64, sp: u64) -> Option<(u64, u64)> {


        let id = crate::common::x86::initial_local_apic_id()?;

        use core::ops::Deref;

        let mut threads = self.shared.threads.lock();

        let from_handle = threads.get_mut(&id)?;
        *from_handle.context.lock() = Some(Context {
            ip,
            sp,
        });

        let mut next = self.shared.next.lock();
        if next.is_empty() {
            *next = self.shared.all.lock().iter().cloned().collect();
        }

        let next_handle = next.pop()?;
        let context = next_handle.context.lock();

        match context.deref() {
            Some(context) => {
                Some((context.sp, context.ip))
            }
            None => {
                None
            }
        }
    }
}

fn hello() -> ! {
    Kernel::get().actor_system().enter();

    loop {}
}
