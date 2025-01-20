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
    context: crate::common::Mutex<Option<Context>>,
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
    all: crate::common::Mutex<BTreeSet<Arc<Handle>>>,
    next: crate::common::Mutex<Vec<Arc<Handle>>>,
    threads: crate::common::Mutex<BTreeMap<usize, Arc<Handle>>>,
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
                context: crate::common::Mutex::default(),
            });

            shared.all.lock().insert(handle.clone());

            actor.create(()).await.unwrap();

            loop {
                {
                    let mut threads = shared.threads.lock();
                    threads.insert(
                        crate::common::x86::initial_local_apic_id().unwrap(),
                        handle.clone(),
                    );
                }

                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => {
                        {
                            let mut threads = shared.threads.lock();
                            threads.remove(
                                &crate::common::x86::initial_local_apic_id().unwrap(),
                            );
                        }

                        break;
                    }
                };

                actor
                    .handle(Self::HandleContext::<A::Message>::new(message))
                    .await
                    .unwrap();

                {
                    let mut threads = shared.threads.lock();
                    threads.remove(
                        &crate::common::x86::initial_local_apic_id().unwrap(),
                    );
                }
            }

            actor.destroy(()).await.unwrap();

            shared.all.lock().remove(&handle);
        }).unwrap();

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
    pub fn reschedule(&self, sp: u64, ip: u64) -> Option<(u64, Option<fn() -> !>)> {
        let id = crate::common::x86::initial_local_apic_id()?;

        use core::ops::Deref;

        let mut threads = self.shared.threads.lock();
        {
            let from_handle = match threads.get_mut(&id) {
                Some(handle) => handle,
                None => {
                    return None;
                }
            };
            *from_handle.context.lock() = Some(Context {
                ip,
                sp,
            });
        }

        threads.remove(&id);

        let mut next = self.shared.next.lock();
        if next.is_empty() {
            *next = self.shared.all.lock().iter().cloned().collect();
        }

        let next_handle = next.pop()?;

        Kernel::get()
            .logger()
            .writer(|w| write!(w, "next is {}\n", next_handle.identifier));

        threads.insert(id, next_handle.clone());

        let context = next_handle.context.lock();

        if let Some(context) = context.deref() {
            Kernel::get()
                .logger()
                .writer(|w| write!(w, "reuse stack... {:X}\n", context.sp,));
            return Some((context.sp, None));
        }

        let kernel = Kernel::get();
        let mut mapper = kernel.page_table_mapper();

        use x86_64::structures::paging::{Mapper, Page, PageTableFlags};
        use x86_64::PhysAddr;
        use x86_64::VirtAddr;
        use x86_64::structures::paging::{PhysFrame, Size4KiB};
        use crate::kernel::EmptyFrameAllocator;

        let mut stack_address = 0;

        Kernel::get()
            .logger()
            .writer(|w| write!(w, "create new stack... {}\n", next_handle.identifier));

        for stack_frame_identifier in kernel
            .frame_manager()
            .allocate_window(16).unwrap()
        {
            stack_address = kernel
                .frame_manager()
                .translate_frame_identifier(stack_frame_identifier).as_usize();

            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                (stack_address).try_into().unwrap(),
            ));

            unsafe {
                mapper
                    .map_to(
                        page,
                        PhysFrame::from_start_address(PhysAddr::new(
                            stack_address.try_into().unwrap(),
                        ))
                            .unwrap(),
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        &mut EmptyFrameAllocator,
                    )
                    .expect("Hello World")
                    .flush();
            }
        }

        Some((stack_address as _, Some(hello as _)))
    }
}

fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    Kernel::get().actor_system().enter().unwrap();

    loop {}
}
