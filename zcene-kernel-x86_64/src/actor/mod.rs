use crate::common::println;
use crate::kernel::Kernel;
use core::fmt::Write;
use core::marker::PhantomData;
use zcene_core::actor;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorCommonHandleContext, ActorEnterError, ActorMessage,
    ActorMessageChannel, ActorMessageChannelAddress, ActorSpawnError,
};
use zcene_core::future::runtime::{FutureRuntimeHandler, FutureRuntimeReference};
use ztd::Constructor;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor, Debug)]
pub struct ActorSpecification {
    preemptive: bool,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ExecutionUnitIdentifier = usize;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type ActorIdentifier = usize;

use crate::kernel::EmptyFrameAllocator;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::paging::{Mapper, Page, PageTableFlags};
use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::PhysAddr;
use x86_64::VirtAddr;

struct Context {
    stack_pointer: u64,
}

#[derive(Debug, Default)]
pub struct Handle {
    identifier: ActorIdentifier,
    stack_pointer: AtomicU64,
}

#[derive(Debug)]
pub enum Thread {
    Cooperative { stack_pointer: AtomicU64 },
    Preemptive { actor: Arc<Handle> },
}

impl Thread {
    pub fn is_cooperative(&self) -> bool {
        matches!(self, Self::Cooperative { .. })
    }

    pub fn is_preemtive(&self) -> bool {
        matches!(self, Self::Preemptive { .. })
    }
}

#[derive(Default)]
pub struct Scheduler {
    queue: VecDeque<Thread>,
    threads: BTreeMap<usize, Arc<Handle>>,
    stacks: BTreeSet<u64>,
}

use zcene_kernel::common::memory::VirtualMemoryAddress;

pub type ActorExecutionContextIdentifier = usize;

use ztd::Method;

#[derive(Constructor, Debug)]
pub struct ActorExecutionContext {
    identifier: ActorExecutionContextIdentifier,
    stack_pointer: VirtualMemoryAddress,
}

pub type ActorExecutionContextReference<H> =
    Arc<ActorExecutionContext, <H as zcene_core::actor::ActorHandler>::Allocator>;

#[derive(Default, Debug)]
pub struct ActorQueuePreemptionSchedulerStrategy {
    queue: VecDeque<Arc<ActorExecutionContext>>,
}

/*impl<H> ActorPreemptionScheduler<H> for ActorQueuePreemptionSchedulerStrategy
where
    H: zcene_core::actor::ActorHandler,
{
}*/

pub trait ActorPreemptionSchedulerStrategy<H>
where
    H: zcene_core::actor::ActorHandler,
{
    fn r#continue(&mut self, execution_context: ActorExecutionContextReference<H>) {}

    fn r#break(&mut self) {}
}

impl Scheduler {
    fn report_handle(&mut self, handle: Option<Arc<Handle>>) {
        let id = crate::architecture::initial_local_apic_id().unwrap();

        let current = self.threads.get(&id).cloned().map(|x| x.identifier);

        if let Some(ref handle) = handle {
            self.threads.insert(id, handle.clone());
        } else {
            self.threads.remove(&id);
        }
    }
}

use zcene_kernel::synchronization::Mutex;

#[derive(Default)]
pub struct Shared {
    scheduler: Mutex<Scheduler>,
    identifier_counter: AtomicUsize,
}

use alloc::sync::Arc;

#[derive(Constructor)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
    //S: ActorPreemptionScheduler<Self>,
{
    future_runtime: FutureRuntimeReference<H>,
    //preemption_scheduler: S,
    //scheduler: Arc<S>
    shared: Arc<Shared>,
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

use x86_64::instructions::interrupts;
use x86_64::instructions::interrupts::without_interrupts;

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

        let mut pinned = pin!(self.actor.handle(ActorCommonHandleContext::new(message)));

        without_interrupts(|| {
            //preemption_scheduler.r#continue(handle.clone());
            shared.scheduler.lock().report_handle(Some(handle.clone()));
        });

        let result = pinned.as_mut().poll(context);

        without_interrupts(|| {
            //preemption_scheduler.r#break();
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
        = ActorCommonHandleContext<M>
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

        let identifier = self
            .shared
            .identifier_counter
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);

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
                    None => break,
                };

                (ActorHandleExecutor {
                    actor: &mut actor,
                    message,
                    shared: shared.clone(),
                    handle: handle.clone(),
                    handler: PhantomData::<H>,
                })
                .await;
            }

            println!("exit...");
        });

        Ok(reference)
    }

    fn enter(&self) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

use x86_64::registers::rflags::RFlags;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::PrivilegeLevel;

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
    /*pub fn reschedule2(&self, stack_pointer: u64) -> u64 {
        let mut scheduler = self.shared.scheduler.lock();

        let id = crate::architecture::initial_local_apic_id().unwrap();

        let next_handle = scheduler.queue.pop_front();

        let current_handle = match scheduler.threads.get(&id).cloned() {
            Some(current_handle) => {
                scheduler.threads.remove(&id);
                current_handle.stack_pointer.store(stack_pointer, Ordering::SeqCst);
                scheduler.queue.push_back(current_handle.clone());

                Some(current_handle)
            },
            None => {
                scheduler.stacks.insert(stack_pointer);

                None
            },
        };

        match next_handle {
            Some(next_handle) => {
                scheduler.threads.insert(id, next_handle.clone());
                next_handle.stack_pointer.load(Ordering::SeqCst)
            },
            None => {
                match scheduler.stacks.pop_first() {
                    Some(stack_pointer) => stack_pointer,
                    None => create_new_stack(Kernel::get().allocate_stack()),
                }
            }
        }
    }*/

    pub fn reschedule(&self, stack_pointer: u64) -> u64 {
        let mut scheduler = self.shared.scheduler.lock();

        let id = crate::architecture::initial_local_apic_id().unwrap();

        match scheduler.threads.get(&id).cloned() {
            Some(current_handle) => {
                scheduler.threads.remove(&id);

                current_handle
                    .stack_pointer
                    .store(stack_pointer, Ordering::SeqCst);
                scheduler.queue.push_back(Thread::Preemptive {
                    actor: current_handle.clone(),
                });

                if scheduler
                    .queue
                    .iter()
                    .filter(|x| x.is_cooperative())
                    .count()
                    < 1
                {
                    scheduler.queue.push_back(Thread::Cooperative {
                        stack_pointer: AtomicU64::new(create_new_stack(
                            Kernel::get().allocate_stack(),
                        )),
                    });
                }
            }
            None => {
                scheduler.queue.push_back(Thread::Cooperative {
                    stack_pointer: AtomicU64::new(stack_pointer),
                });
            }
        };

        println!(
            "{:?}",
            scheduler.queue.iter().map(|x| {
                match x {
                    Thread::Cooperative { .. } => "coop",
                    Thread::Preemptive { .. } => "preempt",
                }
            })
        );

        match scheduler.queue.pop_front() {
            Some(Thread::Preemptive { actor: next_handle }) => {
                scheduler.threads.insert(id, next_handle.clone());
                next_handle.stack_pointer.load(Ordering::SeqCst)
            }
            Some(Thread::Cooperative { stack_pointer }) => stack_pointer.load(Ordering::SeqCst),
            None => {
                panic!("HELLO")
            }
        }
    }

    pub fn preempt(&self, stack_pointer: VirtualMemoryAddress) -> VirtualMemoryAddress {
        stack_pointer
    }
}

fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}
