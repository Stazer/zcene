use crate::architecture::current_execution_unit_identifier;
use crate::kernel::future::runtime::KernelFutureRuntimeHandler;
use crate::kernel::future::runtime::KernelFutureRuntimeReference;
use crate::kernel::logger::println;
use crate::kernel::Kernel;
use crate::kernel::TimerActorMessage;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::arch::asm;
use core::arch::naked_asm;
use core::marker::PhantomData;
use core::mem::replace;
use core::task::Waker;
use x86::current::registers::rsp;
use x86::current::rflags::RFlags;
use x86::msr::{rdmsr, wrmsr, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::registers::segmentation::{Segment, SegmentSelector, CS, DS, ES, SS};
use x86_64::PrivilegeLevel;
use zcene_bare::memory::address::PhysicalMemoryAddress;
use zcene_bare::memory::address::VirtualMemoryAddress;
use zcene_bare::memory::address::VirtualMemoryAddressPerspective;
use zcene_bare::memory::region::VirtualMemoryRegion;
use zcene_core::actor::ActorMessageSender;
use zcene_core::actor::{
    Actor, ActorAddressReference, ActorCommonHandleContext, ActorDiscoveryHandler, ActorEnterError,
    ActorHandler, ActorMailbox, ActorMessage, ActorMessageChannel, ActorMessageChannelAddress,
    ActorSpawnError,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use zcene_core::future::FutureExt;
use ztd::Constructor;

pub use super::*;

#[derive(Clone, Debug, Default)]
#[repr(C, packed)]
pub struct ActorDeadlineStackFrame {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type KernelActorInstructionRegion = VirtualMemoryRegion;

pub struct KernelActorSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    actor: A,
    execution_mode: KernelActorExecutionMode,
    instruction_region: KernelActorInstructionRegion,
    handler: PhantomData<H>,
}

impl<A, H> KernelActorSpawnSpecification<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    pub fn new(
        actor: A,
        execution_mode: KernelActorExecutionMode,
        instruction_region: KernelActorInstructionRegion,
    ) -> Self {
        Self {
            actor,
            execution_mode,
            instruction_region,
            handler: PhantomData::<H>,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Constructor)]
pub struct KernelActorHandler {
    future_runtime: KernelFutureRuntimeReference,
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

    type SpawnSpecification<A>
        = KernelActorSpawnSpecification<A, Self>
    where
        A: Actor<Self>;

    type EnterSpecification = ();

    fn allocator(&self) -> &Self::Allocator {
        self.future_runtime.handler().allocator()
    }

    fn spawn<A>(
        &self,
        specification: Self::SpawnSpecification<A>,
    ) -> Result<ActorAddressReference<A, Self>, ActorSpawnError>
    where
        A: Actor<Self>,
    {
        let (sender, receiver) = ActorMessageChannel::<A::Message>::new_unbounded();

        let reference = ActorAddressReference::<A, Self>::try_new_in(
            <Self as ActorHandler>::Address::new(sender),
            self.allocator().clone(),
        )?;

        self.future_runtime.spawn(ActorExecutor {
            r#type: match specification.execution_mode {
                KernelActorExecutionMode::Privileged => ActorExecutorType::InternalPrivileged {
                    state: ActorExecutorState::Created(specification.actor),
                },
                KernelActorExecutionMode::Unprivileged => ActorExecutorType::InternalUnprivileged {
                    state: ActorExecutorState::Created(Box::new(specification.actor)),
                },
            },
            receiver,
        });

        Ok(reference)
    }

    fn enter(&self, specification: Self::EnterSpecification) -> Result<(), ActorEnterError> {
        self.future_runtime.run();

        Ok(())
    }
}

use core::future::Future;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};
use zcene_core::actor::ActorMessageChannelReceiver;

use zcene_core::actor::ActorCreateError;

#[naked]
pub unsafe extern "C" fn actor_system_call_entry_point() {
    naked_asm!(
        // Load kernel stack pointer
        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",
        // First argument is passed from system call itself
        // Prepare second argument
        "pop rsi",
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        // At this stage it looks like we execute from within the future
        "cld",
        "jmp actor_system_call_restore",
    )
}

#[no_mangle]
#[inline(never)]
unsafe extern "C" fn actor_system_call_restore(
    // TODO: check size
    system_call: &ActorExecutorStageSystemCall<Result<(), ActorCreateError>>,
    event: &mut ActorExecutorStageEvent<Result<(), ActorCreateError>>,
) {
    *event = ActorExecutorStageEvent::SystemCall(system_call.clone());
}

#[naked]
pub unsafe extern "C" fn actor_deadline_entry_point() {
    naked_asm!(
        // Save context
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push r11",
        "push r10",
        "push r9",
        "push r8",
        "push rbp",
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push rbx",
        "push rax",
        // Prepare first argument
        "mov rdi, rsp",
        // Load kernel stack pointer
        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",
        // Prepare second argument
        "pop rsi",
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        // At this stage it looks like we execute from within the future
        "cld",
        "jmp actor_deadline_restore",
    )
}

#[no_mangle]
#[inline(never)]
unsafe extern "C" fn actor_deadline_restore(
    stack: &ActorDeadlineStackFrame,
    event: &mut ActorExecutorStageEvent<Result<(), ActorCreateError>>,
) {
    *event = ActorExecutorStageEvent::Preemption {
        stack: stack.clone(),
    };
}

#[derive(Debug)]
pub enum ActorExecutorStageSystemCall<T> {
    Poll(Poll<T>),
}

impl<T> Clone for ActorExecutorStageSystemCall<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Poll(poll) => Self::Poll(poll.clone()),
        }
    }
}

#[derive(Debug)]
pub enum ActorExecutorStageEvent<T> {
    Ready,
    SystemCall(ActorExecutorStageSystemCall<T>),
    Preemption { stack: ActorDeadlineStackFrame },
}

enum ActorExecutorState<A, M> {
    Created(A),
    CreatedPreempted {
        actor: A,
        stack_frame: ActorDeadlineStackFrame,
    },
    RunningReceive(A),
    RunningHandle(A, M),
    Destroy(A),
    Destroyed,
    Unknown,
}

pub enum ActorExecutorType<A>
where
    A: Actor<KernelActorHandler>,
{
    InternalPrivileged {
        state: ActorExecutorState<A, A::Message>,
    },
    InternalUnprivileged {
        state: ActorExecutorState<Box<A>, A::Message>,
    },
}

#[derive(Constructor)]
#[pin_project::pin_project]
pub struct ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
    r#type: ActorExecutorType<A>,
    receiver: ActorMessageChannelReceiver<A::Message>,
}

impl<A> ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
    fn poll_internal_privileged(
        state: &mut ActorExecutorState<A, A::Message>,
        context: &mut Context<'_>,
        receiver: &mut ActorMessageChannelReceiver<A::Message>,
    ) -> Poll<<Self as Future>::Output> {
        loop {
            match replace(state, ActorExecutorState::Unknown) {
                ActorExecutorState::Created(mut actor) => {
                    let result = {
                        let mut pinned = pin!(actor.create(()));
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::Created(actor);
                            return Poll::Pending;
                        }
                        // TODO: handle result
                        Poll::Ready(_result) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }
                }
                ActorExecutorState::CreatedPreempted { .. } => {
                    todo!()
                }
                ActorExecutorState::RunningReceive(mut actor) => {
                    let result = {
                        let mut pinned = pin!(receiver.receive());
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningReceive(actor);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(message)) => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                        }
                        Poll::Ready(None) => {
                            *state = ActorExecutorState::Destroy(actor);
                        }
                    }
                }
                ActorExecutorState::RunningHandle(mut actor, message) => {
                    let result = {
                        let mut pinned =
                            pin!(actor.handle(ActorCommonHandleContext::new(message.clone())));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                            return Poll::Pending;
                        }
                        Poll::Ready(message) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }
                }
                ActorExecutorState::Destroy(mut actor) => {
                    let mut pinned = pin!(actor.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }

                    *state = ActorExecutorState::Destroyed;
                }
                ActorExecutorState::Destroyed => return Poll::Ready(()),
                ActorExecutorState::Unknown => return Poll::Ready(()),
            }
        }

        Poll::Ready(())
    }

    extern "C" fn execute(actor: &mut A) -> ! {
        let mut context = Context::from_waker(Waker::noop());

        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => ActorExecutorStageSystemCall::Poll(Poll::Pending),
            Poll::Ready(result) => ActorExecutorStageSystemCall::Poll(Poll::Ready(result)),
        };

        unsafe {
            asm!(
                "mov rdi, {}",
                "syscall",

                in(reg) &result,

                options(noreturn),
            )
        }
    }

    #[naked]
    unsafe fn system_return<T>(
        user_stack: u64,                         // rdi
        function: u64,                           // rsi
        actor: &mut A,                           // rdx
        result: &mut ActorExecutorStageEvent<T>, // rcx
        size: u64,                               // r8
                                                 //rflags: RFlags, // r9
    ) {
        naked_asm!(
            // Save callee-saved registers
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            // Save stage event address
            "push rcx",
            // Store kernel stack pointer
            "mov rcx, 0xC0000102",
            "mov rax, rsp",
            "mov rdx, rax",
            "shr rdx, 32",
            "wrmsr",
            // Perform system return
            "push 32 | 3",
            "push rdi",
            "push 0x200",
            "push 24 | 3",
            "push rsi",
            "iretq",
        )
    }

    #[naked]
    unsafe extern "C" fn r#continue<T>(
        _result: &mut ActorExecutorStageEvent<T>, // rdi
        _stack_frame: &ActorDeadlineStackFrame,
    ) {
        naked_asm!(
            // Save callee-saved registers
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            // Save stage event address
            "push rdi",
            // Store kernel stack pointer
            "mov rcx, 0xC0000102",
            "mov rax, rsp",
            "mov rdx, rax",
            "shr rdx, 32",
            "wrmsr",
            // Load preempted stack frame
            "mov rsp, rsi",
            // Restore context
            "pop rax",
            "pop rbx",
            "pop rcx",
            "pop rdx",
            "pop rsi",
            "pop rdi",
            "pop rbp",
            "pop r8",
            "pop r9",
            "pop r10",
            "pop r11",
            "pop r12",
            "pop r13",
            "pop r14",
            "pop r15",
            "iretq",
        )
    }

    fn poll_internal_unprivileged(
        state: &mut ActorExecutorState<Box<A>, A::Message>,
        context: &mut Context<'_>,
        receiver: &mut ActorMessageChannelReceiver<A::Message>,
    ) -> Poll<<Self as Future>::Output> {
        loop {
            match replace(state, ActorExecutorState::Unknown) {
                ActorExecutorState::Created(mut actor) => {
                    let user_stack = Kernel::get()
                        .memory_manager()
                        .allocate_user_stack()
                        .unwrap()
                        .initial_memory_address()
                        .as_u64();

                    let mut result = ActorExecutorStageEvent::<Result<(), ActorCreateError>>::Ready;

                    Kernel::get()
                        .interrupt_manager()
                        .reset_oneshot(core::time::Duration::from_millis(100));

                    unsafe {
                        Self::system_return::<Result<(), ActorCreateError>>(
                            user_stack,
                            Self::execute as _,
                            &mut actor,
                            &mut result,
                            size_of::<ActorExecutorStageEvent<Result<(), ActorCreateError>>>()
                                as u64,
                            //RFlags::empty() | RFlags::FLAGS_IF,
                        );
                    };

                    match result {
                        ActorExecutorStageEvent::Ready => {
                            return Poll::Pending;
                        }
                        ActorExecutorStageEvent::Preemption { mut stack } => {
                            //*state = ActorExecutorState::CreatedPreempted { actor, stack_frame: stack };
                            //context.waker().clone().wake_by_ref();

                            crate::kernel::logger::println!("preempt...!");

                            Kernel::get()
                                .interrupt_manager()
                                .reset_oneshot(core::time::Duration::from_millis(100));
                            Kernel::get()
                                .interrupt_manager()
                                .notify_local_end_of_interrupt();

                            let mut result2 =
                                ActorExecutorStageEvent::<Result<(), ActorCreateError>>::Ready;

                            unsafe { Self::r#continue(&mut result2, &mut stack) }

                            crate::kernel::logger::println!("finish {:?}", result2);

                            loop {}

                            return Poll::Pending;
                        }
                        ActorExecutorStageEvent::SystemCall(_) => {
                            crate::kernel::logger::println!("syscall!");

                            *state = ActorExecutorState::RunningReceive(actor);
                            crate::kernel::logger::println!("syscall after!");
                        }
                    }
                }
                ActorExecutorState::CreatedPreempted { actor, stack_frame } => {
                    crate::kernel::logger::println!("continue... {:X?}", stack_frame);

                    Kernel::get()
                        .interrupt_manager()
                        .reset_oneshot(core::time::Duration::from_millis(100));

                    let mut result = ActorExecutorStageEvent::<Result<(), ActorCreateError>>::Ready;

                    unsafe {
                        Self::r#continue(&mut result, &stack_frame);
                    }

                    crate::kernel::logger::println!("finished! {:?}", result);

                    loop {}
                }
                ActorExecutorState::RunningReceive(mut actor) => {
                    let result = {
                        let mut pinned = pin!(receiver.receive());
                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningReceive(actor);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(message)) => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                        }
                        Poll::Ready(None) => {
                            *state = ActorExecutorState::Destroy(actor);
                        }
                    }
                }
                ActorExecutorState::RunningHandle(mut actor, message) => {
                    let result = {
                        let mut pinned =
                            pin!(actor.handle(ActorCommonHandleContext::new(message.clone())));

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            *state = ActorExecutorState::RunningHandle(actor, message);
                            return Poll::Pending;
                        }
                        Poll::Ready(message) => {
                            *state = ActorExecutorState::RunningReceive(actor);
                        }
                    }
                }
                ActorExecutorState::Destroy(mut actor) => {
                    let mut pinned = pin!(actor.destroy(()));

                    loop {
                        // TODO
                        if let Poll::Ready(_) = pinned.as_mut().poll(context) {
                            break;
                        }
                    }

                    *state = ActorExecutorState::Destroyed;
                }
                ActorExecutorState::Destroyed => return Poll::Ready(()),
                ActorExecutorState::Unknown => {
                    println!("unknown!");

                    return Poll::Ready(());
                }
            }
        }

        Poll::Ready(())
    }
}

impl<A> Future for ActorExecutor<A>
where
    A: Actor<KernelActorHandler>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            receiver, r#type, ..
        } = &mut *self;

        match r#type {
            ActorExecutorType::InternalPrivileged { state } => {
                Self::poll_internal_privileged(state, context, receiver)
            }
            ActorExecutorType::InternalUnprivileged { state } => {
                Self::poll_internal_unprivileged(state, context, receiver)
            }
        }
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
