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
use x86_64::structures::paging::{Mapper, Page, PageTableFlags};
use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::structures::paging::{PhysFrame, Size4KiB};
use crate::kernel::EmptyFrameAllocator;

struct Context {
    stack_pointer: u64,
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
pub struct Scheduler {
    all: BTreeSet<Arc<Handle>>,
    next: Vec<Arc<Handle>>,
    threads: BTreeMap<usize, Arc<Handle>>,
}

impl Scheduler {
    fn report_handle(&mut self, handle: Option<Arc<Handle>>) {
        let id = crate::common::x86::initial_local_apic_id().unwrap();

        let current = self.threads.get(&id).cloned().map(|x| x.identifier);

        if let Some(ref handle) = handle {
            self.threads.insert(id, handle.clone());
        } else {
            self.threads.remove(&id);
        }

       /* Kernel::get()
            .logger()
            .writer(|w| write!(w, "{:?} -> {:?}\n", current, handle.map(|x| x.identifier)));*/
    }
}

#[derive(Default)]
pub struct Shared {
    scheduler: spin::mutex::SpinMutex<Scheduler>,
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

use core::future::Future;
use core::pin::{pin, Pin};
use core::task::Poll;
use pin_project::pin_project;

#[pin_project]
pub struct ActorHandleExecutor<'a, A, H>
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    actor: &'a mut A,
    message: A::Message,
    shared: Arc<Shared>,
    handle: Arc<Handle>,
    handler: PhantomData<H>,
}

use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::interrupts;

impl<'a, A, H> Future for ActorHandleExecutor<'a, A, H>
where
    A: Actor<ActorHandler<H>>,
    H: FutureRuntimeHandler,
{
    type Output = Result<(), actor::ActorHandleError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let handle = self.handle.clone();
        let shared = self.shared.clone();
        let message = self.message.clone();

        let mut pinned = pin!(self.actor.handle(FutureRuntimeActorHandleContext::new(message)));

        without_interrupts(|| {
            shared.scheduler.lock().report_handle(Some(handle.clone()));
        });

        let result = pinned.as_mut().poll(context);

        without_interrupts(|| {
            shared.scheduler.lock().report_handle(None);
        });

        result
    }
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

        let handle = Arc::new(Handle {
            identifier,
            context: crate::common::Mutex::default(),
        });

        self.future_runtime.spawn(async move {
            let mut actor = actor;

            {
                let mut scheduler = shared.scheduler.lock();
                scheduler.all.insert(handle.clone());
            }

            loop {
                /*Kernel::get()
                    .logger()
                    .writer(|w| write!(w, "recv...\n"));*/

                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => continue,
                };

                /*Kernel::get()
                    .logger()
                    .writer(|w| write!(w, "recv next\n"));*/

                (ActorHandleExecutor {
                    actor: &mut actor,
                    message,
                    shared: shared.clone(),
                    handle: handle.clone(),
                    handler: PhantomData::<H>,
                }).await;

                /*Kernel::get()
                    .logger()
                    .writer(|w| write!(w, "exec finish\n"));*/
            }
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        use zcene_core::future::runtime::FutureRuntimeQueue;
        use zcene_core::future::runtime::FutureRuntimeYielder;
        use futures::task::waker_ref;

        loop {
            let task = match self.future_runtime.handler().queue().dequeue() {
                Some(task) => task,
                None => {
                    self.future_runtime.handler().yielder().r#yield();
                    continue;
                }
            };

            let mut future_slot = task.slot().lock();

            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let context = &mut core::task::Context::from_waker(&waker);

                if future.as_mut().poll(context).is_pending() {
                    *future_slot = Some(future);

                    /*Kernel::get()
                        .logger()
                        .writer(|w| write!(w, "poll future pending\n"));*/
                } else {
                    /*Kernel::get()
                        .logger()
                        .writer(|w| write!(w, "poll future done\n"));*/
                }
            }
        }

        //self.shared.threads.lock();

        //self.future_runtime.run();

        Ok(())
    }
}

use x86_64::PrivilegeLevel;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::registers::rflags::RFlags;

#[inline(never)]
extern "C" fn create_new_stack(mut new_stack_pointer: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "mov rbx, rsp",

            "mov rsp, {new_stack_pointer}",

            "push {stack_segment:r}",
            "push {new_stack_pointer}",
            "push {rflags}",
            "push {code_segment:r}",
            "push {new_instruction_pointer}",

            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",
            "push 0",

            "mov {new_stack_pointer}, rsp",

            "mov rsp, rbx",

            rflags = in(reg) RFlags::RESUME_FLAG.bits(),
            new_instruction_pointer = in(reg) VirtAddr::new(hello as _).as_u64(),
            new_stack_pointer = inout(reg) new_stack_pointer,
            code_segment = in(reg) SegmentSelector::new(1, PrivilegeLevel::Ring0).0,
            stack_segment = in(reg) SegmentSelector::new(2, PrivilegeLevel::Ring0).0,
        )
    }

    new_stack_pointer
}

impl<H> ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    pub fn reschedule(&self, stack_pointer: u64) -> u64 {
        let mut scheduler = self.shared.scheduler.lock();

        let id = crate::common::x86::initial_local_apic_id().unwrap();

        let current_handle = match scheduler.threads.get(&id).cloned() {
            Some(handle) => {
                *handle.context.lock() = Some(Context {
                    stack_pointer,
                });

                scheduler.report_handle(None);

                handle
            }
            None => {
                return stack_pointer;
            }
        };

        stack_pointer

        /*if scheduler.next.is_empty() {
            scheduler.next = scheduler.all.iter().cloned().collect();
        }

        let next_handle = scheduler.next.pop()?;

        scheduler.report_handle(Some(next_handle.clone()));*/

        //create_new_stack(Kernel::get().allocate_stack())

        //let new_stack_pointer = create_new_stack(Kernel::get().allocate_stack());

        //stack_pointer

        /*let mut scheduler = self.shared.scheduler.lock();

        let id = crate::common::x86::initial_local_apic_id()?;

        let current_handle = {
            match scheduler.threads.get(&id).cloned() {
                Some(handle) => {
                    *handle.context.lock() = Some(Context {
                        ip,
                        sp,
                    });

                    scheduler.report_handle(Some(handle.clone()));

                    Some(handle)
                }
                None => {
                    None
                }
            }
        };

        if scheduler.next.is_empty() {
            scheduler.next = scheduler.all.iter().cloned().collect();
        }

        let next_handle = scheduler.next.pop()?;

        scheduler.report_handle(Some(next_handle.clone()));

        let context = next_handle.context.lock();

        use core::ops::Deref;

        if let Some(context) = context.deref() {
            return Some((context.sp, None));
        }

        let kernel = Kernel::get();
        let mut mapper = kernel.page_table_mapper();

        /*Kernel::get()
            .logger()
            .writer(|w| write!(w, "create new stack {:X} for {}\n", stack_address, next_handle.identifier));*/

        Some((stack_address as _, Some(hello as _)))*/
    }
}

fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}
