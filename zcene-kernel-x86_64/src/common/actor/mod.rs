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

use crate::common::println;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug)]
pub struct ActorSpecification {
    preemptive: bool,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorIdentifier = usize;

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::{Ordering, AtomicU64};
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
    stack_pointer: AtomicU64,
}

#[derive(Default)]
pub struct SchedulerQueue {
    all: VecDeque<Arc<Handle>>,
}

#[derive(Default)]
pub struct Scheduler {
    queue: VecDeque<Arc<Handle>>,
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
            stack_pointer: AtomicU64::default(),
        });

        self.future_runtime.spawn(async move {
            let mut actor = actor;

            loop {
                let message = match receiver.receive().await {
                    Some(message) => message,
                    None => continue,
                };

                (ActorHandleExecutor {
                    actor: &mut actor,
                    message,
                    shared: shared.clone(),
                    handle: handle.clone(),
                    handler: PhantomData::<H>,
                }).await;
            }
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

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

            rflags = in(reg) (RFlags::RESUME_FLAG | RFlags::INTERRUPT_FLAG).bits(),
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
            Some(current_handle) => current_handle,
            None => return stack_pointer,
        };

        scheduler.threads.remove(&id);
        current_handle.stack_pointer.store(stack_pointer, Ordering::SeqCst);

        let next_handle = scheduler.queue.pop_front();

        scheduler.queue.push_back(current_handle.clone());

        match next_handle {
            Some(next_handle) => next_handle.stack_pointer.load(Ordering::SeqCst),
            None => {
                create_new_stack(Kernel::get().allocate_stack())
            }
        }
    }
}

fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}
