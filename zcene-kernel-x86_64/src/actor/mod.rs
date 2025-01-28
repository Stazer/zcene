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

#[derive(Debug, Default)]
pub struct Handle {
    stack_pointer: AtomicU64,
}

#[derive(Debug)]
pub enum Thread {
    Cooperative {
        stack_pointer: AtomicU64,
    },
    Preemptive {
        actor: Arc<Handle>,
        stack_pointer_memory_address: u32,
    },
}

impl Thread {
    pub fn is_cooperative(&self) -> bool {
        matches!(self, Self::Cooperative { .. })
    }

    pub fn is_preemptive(&self) -> bool {
        matches!(self, Self::Preemptive { .. })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ActorThreadType {
    Cooperative,
    Preemptive,
}

#[derive(Debug)]
pub struct ActorThread {
    r#type: ActorThreadType,
    stack_pointer: Option<VirtualMemoryAddress>,
}

#[derive(Default)]
pub struct ActorThreadScheduler {
    queue: VecDeque<Thread>,
    threads: BTreeMap<usize, Arc<Handle>>,
    queue2: VecDeque<ActorThread>,
    threads2: BTreeMap<ExecutionUnitIdentifier, ActorThread>,
}

impl ActorThreadScheduler {
    pub fn r#break(
        &mut self,
        execution_unit_identifier: ExecutionUnitIdentifier,
        stack_pointer: VirtualMemoryAddress
    ) {
        match self.threads2.remove(&execution_unit_identifier) {
            Some(mut thread) => {
                thread.stack_pointer = Some(stack_pointer);
                self.queue2.push_back(thread);

                if self
                    .queue2
                    .iter()
                    .filter(|x| matches!(x.r#type, ActorThreadType::Cooperative))
                    .count()
                    < 1
                {
                    self.queue2.push_back(ActorThread {
                        r#type: ActorThreadType::Cooperative,
                        stack_pointer: None,
                    });
                }
                /*match thread.r#type {
                   ActorThreadType::Preemptive {
                       self.queue2.push_back(ActorThread {
                           r#type: ActorThreadType::
                       })
                   }
                   ActorThreadType::Cooperative {

                   }
                }*/

                /*thread.stack_pointer = Some(stack_pointer);
                thread.r#type = ActorThreadType::Preemptive;

                self.queue2.push_back(thread);

                if self
                    .queue2
                    .iter()
                    .filter(|x| matches!(x.r#type, ActorThreadType::Cooperative))
                    .count()
                    < 1
                {
                    self.queue2.push_back(ActorThread {
                        r#type: ActorThreadType::Cooperative,
                        stack_pointer: None,
                    });
                }*/
            }
            None => {
                /*self.queue2.push_back(ActorThread {
                    r#type: ActorThreadType::Cooperative,
                    stack_pointer: stack_pointer,
                });*/
            }
        }

        /*match self.threads.remove(&execution_unit_identifier) {
            Some(current_handle) => {
                current_handle
                    .stack_pointer
                    .store(stack_pointer.as_u64(), Ordering::SeqCst);

                self.queue.push_back(Thread::Preemptive {
                    actor: current_handle.clone(),
                    stack_pointer_memory_address: 0,
                });

                if self
                    .queue
                    .iter()
                    .filter(|x| x.is_cooperative())
                    .count()
                    < 1
                {
                    self.queue.push_back(Thread::Cooperative {
                        stack_pointer: AtomicU64::new(create_new_stack(
                            Kernel::get().allocate_stack(),
                        )),
                    });
                }
            }
            None => {
                self.queue.push_back(Thread::Cooperative {
                    stack_pointer: AtomicU64::new(stack_pointer.as_u64()),
                });
            }
        }*/
    }

    pub fn r#continue(
        &mut self,
        execution_unit_identifier: ExecutionUnitIdentifier,
        next: Option<Thread>,
        next2: Option<ActorThread>,
    ) -> VirtualMemoryAddress {
        match next2 {
            Some(thread) => {
                let stack_pointer = thread.stack_pointer.unwrap_or_else(|| {
                    println!("create new stack 0...");
                    VirtualMemoryAddress::from(create_new_stack(
                        Kernel::get().allocate_stack(),
                    ))
                });

                self.threads2.insert(execution_unit_identifier, thread);

                return stack_pointer
            }
            None => {
                println!("create new stack 1...");
                return VirtualMemoryAddress::from(create_new_stack(
                    Kernel::get().allocate_stack(),
                ))
            }
        }

        /*match next {
            Some(Thread::Preemptive { actor: next_handle, .. }) => {
                self.threads.insert(execution_unit_identifier, next_handle.clone());
                VirtualMemoryAddress::from(next_handle.stack_pointer.load(Ordering::SeqCst))
            }
            Some(Thread::Cooperative { stack_pointer }) => VirtualMemoryAddress::from(stack_pointer.load(Ordering::SeqCst)),
            None => {
                VirtualMemoryAddress::from(create_new_stack(
                    Kernel::get().allocate_stack(),
                ))
            }
        }*/
    }

    fn report_handle(&mut self, handle: Option<Arc<Handle>>) {
        let id = crate::architecture::initial_local_apic_id().unwrap();

        if let Some(ref handle) = handle {
            self.threads.insert(id, handle.clone());

            self.threads2.insert(id, ActorThread {
                r#type: ActorThreadType::Preemptive,
                stack_pointer: None,
            });
        } else {
            self.threads.remove(&id);

            self.threads2.insert(id, ActorThread {
                r#type: ActorThreadType::Cooperative,
                stack_pointer: None,
            });
        }
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
        let scheduler = self.scheduler.clone();
        let handle = self.handle.clone();
        let message = self.message.clone();

        let mut pinned = pin!(self.actor.handle(ActorCommonHandleContext::new(message)));

        without_interrupts(|| {
            scheduler.lock().report_handle(Some(handle.clone()));
        });

        let result = pinned.as_mut().poll(context);

        without_interrupts(|| {
            scheduler.lock().report_handle(None);
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

        let handle = Arc::new(Handle {
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
                    scheduler: scheduler.clone(),
                    handle: handle.clone(),
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
    pub fn reschedule(&self, stack_pointer: u64) -> u64 {
        let mut scheduler = self.scheduler.lock();

        use crate::architecture::current_execution_unit_identifier;

        let thread = scheduler.queue2.pop_front();
        let next = scheduler.queue.pop_front();

        scheduler.r#break(
            current_execution_unit_identifier(),
            VirtualMemoryAddress::from(stack_pointer),
        );

        scheduler.r#continue(
            current_execution_unit_identifier(),
            next,
            thread,
        ).as_u64()
    }
}

fn hello() -> ! {
    x86_64::instructions::interrupts::enable();

    if Kernel::get().actor_system().enter().is_err() {
        loop {}
    }

    loop {}
}
