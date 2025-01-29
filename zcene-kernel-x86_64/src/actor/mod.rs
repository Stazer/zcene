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

mod actor_thread;
mod actor_thread_type;

pub use actor_thread::*;
pub use actor_thread_type::*;

#[derive(Default)]
pub struct ActorThreadScheduler {
    queue: VecDeque<ActorThread>,
    threads: BTreeMap<ExecutionUnitIdentifier, ActorThread>,
}

impl ActorThreadScheduler {
    pub fn r#break(
        &mut self,
        execution_unit_identifier: ExecutionUnitIdentifier,
        stack_pointer: VirtualMemoryAddress
    ) {
        let mut thread = match self.threads.remove(&execution_unit_identifier) {
            Some(thread) => thread,
            None => return,
        };

        thread.set_stack_pointer(Some(stack_pointer));
        self.queue.push_back(thread);

        if self
            .queue
            .iter()
            .filter(|x| matches!(x.r#type(), ActorThreadType::Cooperative))
            .count()
            < 1
        {
            self.queue.push_back(ActorThread::new(
               ActorThreadType::Cooperative,
               None,
            ));
        }
    }

    pub fn r#continue(
        &mut self,
        execution_unit_identifier: ExecutionUnitIdentifier,
        next_thread: Option<ActorThread>,
    ) -> VirtualMemoryAddress {
        let stack_pointer = match next_thread {
            Some(thread) => {
                let stack_pointer = *thread.stack_pointer();
                self.threads.insert(execution_unit_identifier, thread);

                stack_pointer
            }
            None => None
        };

        match stack_pointer {
            Some(stack_pointer) => {
                stack_pointer
            },
            None => {
                VirtualMemoryAddress::from(create_new_stack(
                    Kernel::get().allocate_stack(),
                ))
            }
        }
    }

    fn begin(&mut self, execution_unit_identifier: ExecutionUnitIdentifier) {
        self.threads.insert(execution_unit_identifier, ActorThread::new(
            ActorThreadType::Preemptive ,
            None
        ));
    }

    fn end(&mut self, execution_unit_identifier: ExecutionUnitIdentifier) {
        self.threads.insert(execution_unit_identifier, ActorThread::new(
            ActorThreadType::Cooperative,
            None,
        ));
    }
}


use zcene_kernel::memory::address::VirtualMemoryAddress;

pub type ActorExecutionContextIdentifier = usize;

use ztd::Method;

use zcene_kernel::synchronization::Mutex;

use alloc::sync::Arc;

#[derive(Constructor)]
pub struct ActorHandler<H>
where
    H: FutureRuntimeHandler,
{
    future_runtime: FutureRuntimeReference<H>,
    scheduler: Arc<Mutex<ActorThreadScheduler>>,
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
    scheduler: Arc<Mutex<ActorThreadScheduler>>,
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
        let scheduler = self.scheduler.clone();
        let message = self.message.clone();

        let mut pinned = pin!(self.actor.handle(ActorCommonHandleContext::new(message)));

        use crate::architecture::current_execution_unit_identifier;

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

                (ActorHandleExecutor {
                    actor: &mut actor,
                    message,
                    scheduler: scheduler.clone(),
                    handler: PhantomData::<H>,
                })
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
    pub fn reschedule(&self, stack_pointer: VirtualMemoryAddress) -> VirtualMemoryAddress {
        let mut scheduler = self.scheduler.lock();

        use crate::architecture::current_execution_unit_identifier;

        let next_thread = scheduler.queue.pop_front();

        scheduler.r#break(
            current_execution_unit_identifier(),
            VirtualMemoryAddress::from(stack_pointer),
        );

        scheduler.r#continue(
            current_execution_unit_identifier(),
            next_thread,
        )
    }
}

fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}
