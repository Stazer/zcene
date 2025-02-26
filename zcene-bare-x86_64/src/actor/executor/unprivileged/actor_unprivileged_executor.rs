use crate::actor::{
    ActorUnprivilegedExecutorCreateState, ActorUnprivilegedExecutorCreateStateInner,
    ActorUnprivilegedExecutorDestroyState, ActorUnprivilegedExecutorDestroyStateInner,
    ActorUnprivilegedExecutorHandleState, ActorUnprivilegedExecutorReceiveState,
    ActorUnprivilegedExecutorState, ActorUnprivilegedStageExecutorContext,
    ActorUnprivilegedStageExecutorDeadlinePreemptionContext,
    ActorUnprivilegedStageExecutorDeadlinePreemptionInner, ActorUnprivilegedStageExecutorEvent,
    ActorUnprivilegedStageExecutorSystemCall, ActorUnprivilegedStageExecutorSystemCallContext,
    ActorUnprivilegedStageExecutorSystemCallInner, ActorUnprivilegedStageExecutorSystemCallType,
};
use crate::kernel::logger::println;
use alloc::boxed::Box;
use core::arch::{asm, naked_asm};
use core::future::Future;
use core::marker::PhantomData;
use core::mem::replace;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::ptr::NonNull;
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use pin_project::pin_project;
use zcene_bare::common::As;
use zcene_core::actor::{
    Actor, ActorContextBuilder, ActorCreateError, ActorHandler, ActorMessageChannelReceiver,
};
use ztd::{Constructor, From};

////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ActorUnprivilegedExecutorRegisterArgument {
    fn as_register_value(self) -> usize;
}

impl<'a, A> ActorUnprivilegedExecutorRegisterArgument for &'a mut A {
    fn as_register_value(self) -> usize {
        self as *mut A as _
    }
}

impl<A> ActorUnprivilegedExecutorRegisterArgument for *mut A {
    fn as_register_value(self) -> usize {
        self as _
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! push_callee_saved_registers {
    () => {
        r#"
            push rbx;
            push rbp;
            push r12;
            push r13;
            push r14;
            push r15;
        "#
    };
}

macro_rules! pop_callee_saved_registers {
    () => {
        r#"
            pop r15;
            pop r14;
            pop r13;
            pop r12;
            pop rbp;
            pop rbx;
        "#
    };
}

macro_rules! push_inline_return_address {
    () => {
        r#"
            lea rax, [2f];
            push rax;
        "#
    };
}

macro_rules! push_event_address {
    () => {
        r#"
            push rsi;
        "#
    };
}

macro_rules! push_kernel_stack {
    () => {
        r#"
            mov rax, rsp;
            mov rdx, rsp;
            shr rdx, 32;
            mov rcx, 0xC0000102;
            wrmsr;
        "#
    };
}

macro_rules! emergency_halt {
    () => {
        r#"
            hlt;
        "#
    };
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum ActorUnprivilegedExecutorStageResult {}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct LeakingAllocator;

pub type LeakingBox<T> = Box<T, LeakingAllocator>;

use core::alloc::AllocError;
use core::alloc::Allocator;
use core::alloc::Layout;

unsafe impl Allocator for LeakingAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _data: NonNull<u8>, _layout: Layout) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub enum ActorUnprivilegedStageResult {
    Preempted(),
    Ready,
}

pub trait ActorUnprivilegedExecutorStageHandler<A, H>
where
    A: Actor<H>,
    H: ActorHandler,
{
    fn execute(&self, actor: *mut A, event: &mut ActorUnprivilegedStageExecutorEvent, stack: usize);

    fn preemption_state(
        &self,
        actor: Box<A>,
        context: ActorUnprivilegedStageExecutorContext,
    ) -> ActorUnprivilegedExecutorState<A, H>;

    fn next_state(&self, actor: Box<A>) -> Option<ActorUnprivilegedExecutorState<A, H>>;
}

#[inline(never)]
extern "C" fn execute<A, H>(
    mut actor: *mut A,
    mut event: &mut ActorUnprivilegedStageExecutorEvent,
    stack: u64,
    function: extern "C" fn(*mut A) -> !,
) where
    A: Actor<H>,
    H: ActorHandler,
{
    unsafe {
        asm!(
            push_callee_saved_registers!(),
            push_inline_return_address!(),
            push_event_address!(),
            //
            // Temporarily save current kernel stack
            //
            "mov rax, rsp",
            //
            // Create interrupt frame for returning into unprivileged mode
            //
            "push 32 | 3",
            "push rdx",
            "push 0x200",
            "push 40 | 3",
            "push rcx",
            //
            // Store previously temporarily saved kernel stack
            //
            "mov rdx, rax",
            "shr rdx, 32",
            "mov rcx, 0xC0000102",
            "wrmsr",
            //
            // Perform return
            //
            "iretq",
            emergency_halt!(),
            "2:",
            pop_callee_saved_registers!(),
            in("rdi") actor,
            in("rsi") event,
            in("rdx") stack,
            in("rcx") function,
            clobber_abi("C"),
        )
    }
}

#[inline(never)]
extern "C" fn continue_from_deadline_preemption<A, H>(
    actor: *mut A,
    event: &mut ActorUnprivilegedStageExecutorEvent,
    context: &ActorUnprivilegedStageExecutorDeadlinePreemptionContext,
) where
    A: Actor<H>,
    H: ActorHandler,
{
    unsafe {
        asm!(
            push_callee_saved_registers!(),
            push_inline_return_address!(),
            push_event_address!(),
            push_kernel_stack!(),
            //
            // Restore user stack from context
            //
            "mov rsp, r8",
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
            //
            // Perform return
            //
            "iretq",
            emergency_halt!(),
            //
            // Restore callee-saved registers
            //
            "2:",
            pop_callee_saved_registers!(),
            in("rdi") actor,
            in("rsi") event,
            in("r8") context,
            clobber_abi("C"),
        )
    }
}

#[inline(never)]
extern "C" fn continue_from_system_call<T>(
    actor: T,
    event: &mut ActorUnprivilegedStageExecutorEvent,
    context: &ActorUnprivilegedStageExecutorSystemCallContext,
) where
    T: ActorUnprivilegedExecutorRegisterArgument,
{
    unsafe {
        asm!(
            push_callee_saved_registers!(),
            push_inline_return_address!(),
            push_event_address!(),
            //
            // Temporarily save current kernel stack
            //
            "mov rax, rsp",
            //
            // Move arguments into temporary registers
            //
            "mov r9, rdx",
            "mov r10, rcx",
            push_kernel_stack!(),
            //
            // Load user stack
            //
            "mov rsp, r9",
            "mov rcx, r10",
            "mov r11, r8",
            //
            // Perform return
            //
            "sysretq",
            emergency_halt!(),
            "2:",
            pop_callee_saved_registers!(),
            in("rdi") actor.as_register_value(),
            in("rsi") event,
            in("rdx") context.rsp(),
            in("rcx") context.rip(),
            in("r8") context.rflags(),
            clobber_abi("C"),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedExecutorCreateStageHandler;

impl ActorUnprivilegedExecutorCreateStageHandler {
    extern "C" fn main<A, H>(actor: *mut A) -> !
    where
        A: Actor<H>,
        H: ActorHandler<CreateContext = ()>,
    {
        let actor = match unsafe { actor.as_mut() } {
            Some(actor) => actor,
            None => unsafe { asm!("mov rdi, 0xFFFF", "syscall", options(noreturn, nostack)) },
        };

        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn, nostack)) }
    }
}

impl<A, H> ActorUnprivilegedExecutorStageHandler<A, H>
    for ActorUnprivilegedExecutorCreateStageHandler
where
    A: Actor<H>,
    H: ActorHandler<CreateContext = ()>,
{
    fn execute(
        &self,
        actor: *mut A,
        event: &mut ActorUnprivilegedStageExecutorEvent,
        stack: usize,
    ) {
        execute(actor, event, stack.r#as(), Self::main)
    }

    fn preemption_state(
        &self,
        actor: Box<A>,
        context: ActorUnprivilegedStageExecutorContext,
    ) -> ActorUnprivilegedExecutorState<A, H> {
        ActorUnprivilegedExecutorCreateState::new(actor, Some(context)).into()
    }

    fn next_state(&self, actor: Box<A>) -> Option<ActorUnprivilegedExecutorState<A, H>> {
        Some(ActorUnprivilegedExecutorReceiveState::new(actor).into())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ActorUnprivilegedExecutorDestroyStageHandler;

impl ActorUnprivilegedExecutorDestroyStageHandler {
    extern "C" fn main<A, H>(actor: *mut A) -> !
    where
        A: Actor<H>,
        H: ActorHandler<DestroyContext = ()>,
    {
        let actor = unsafe { LeakingBox::from_raw_in(actor, LeakingAllocator) };

        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.destroy(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe {
            asm!(
                push_callee_saved_registers!(),
                "mov rdi, 0x0",
                "syscall",
                pop_callee_saved_registers!(),
            )
        }

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }
}

impl<A, H> ActorUnprivilegedExecutorStageHandler<A, H>
    for ActorUnprivilegedExecutorDestroyStageHandler
where
    A: Actor<H>,
    H: ActorHandler<DestroyContext = ()>,
{
    fn execute(
        &self,
        actor: *mut A,
        event: &mut ActorUnprivilegedStageExecutorEvent,
        stack: usize,
    ) {
        execute(actor, event, stack.r#as(), Self::main)
    }

    fn preemption_state(
        &self,
        actor: Box<A>,
        context: ActorUnprivilegedStageExecutorContext,
    ) -> ActorUnprivilegedExecutorState<A, H> {
        ActorUnprivilegedExecutorDestroyState::new(actor, Some(context)).into()
    }

    fn next_state(&self, actor: Box<A>) -> Option<ActorUnprivilegedExecutorState<A, H>> {
        None
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[pin_project]
#[derive(Constructor)]
pub struct ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    state: Option<ActorUnprivilegedExecutorState<A, H>>,
    receiver: ActorMessageChannelReceiver<A::Message>,
    context_builder: B,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}

impl<A, B, H> ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    fn enable_deadline(&mut self) {
        if let Some(deadline_in_milliseconds) = self.deadline_in_milliseconds {
            crate::kernel::Kernel::get()
                .interrupt_manager()
                .reset_oneshot(Duration::from_millis(
                    usize::from(deadline_in_milliseconds).r#as(),
                ));
        }
    }

    fn handle<S>(
        &mut self,
        mut actor: Box<A>,
        future_context: &mut Context<'_>,
        stage_context: Option<ActorUnprivilegedStageExecutorContext>,
        handler: S,
    ) -> Option<Poll<()>>
    where
        S: ActorUnprivilegedExecutorStageHandler<A, H>,
    {
        self.enable_deadline();

        let mut event = ActorUnprivilegedStageExecutorEvent::None;

        match stage_context {
            None => {
                let user_stack = crate::kernel::Kernel::get()
                    .memory_manager()
                    .allocate_user_stack()
                    .unwrap()
                    .initial_memory_address()
                    .as_u64();

                handler.execute(Box::as_mut_ptr(&mut actor), &mut event, user_stack.r#as());
            }
            Some(ActorUnprivilegedStageExecutorContext::SystemCall(system_call_context)) => {
                continue_from_system_call(
                    Box::as_mut_ptr(&mut actor),
                    &mut event,
                    &system_call_context,
                );
            }
            Some(ActorUnprivilegedStageExecutorContext::DeadlinePreemption(
                deadline_preemption_context,
            )) => {
                continue_from_deadline_preemption(
                    Box::as_mut_ptr(&mut actor),
                    &mut event,
                    &deadline_preemption_context,
                );
            }
        }

        loop {
            println!("event {:X?}", event);

            match replace(&mut event, ActorUnprivilegedStageExecutorEvent::None) {
                ActorUnprivilegedStageExecutorEvent::None => break,
                ActorUnprivilegedStageExecutorEvent::SystemCall(system_call) => {
                    let ActorUnprivilegedStageExecutorSystemCallInner {
                        r#type,
                        context: system_call_context,
                    } = system_call.into_inner();

                    match r#type {
                        ActorUnprivilegedStageExecutorSystemCallType::Continue => {
                            continue_from_system_call(
                                Box::as_mut_ptr(&mut actor),
                                &mut event,
                                &system_call_context,
                            )
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Preempt => {
                            self.state =
                                Some(handler.preemption_state(actor, system_call_context.into()));

                            future_context.waker().wake_by_ref();

                            return Some(Poll::Pending);
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(Poll::Pending) => {
                            todo!()
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(Poll::Ready(())) => {
                            self.state = handler.next_state(actor);
                            break;
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Unknown(_) => {
                            // error
                            todo!()
                        }
                    }
                }
                ActorUnprivilegedStageExecutorEvent::DeadlinePreemption(deadline_preemption) => {
                    let ActorUnprivilegedStageExecutorDeadlinePreemptionInner {
                        context: deadline_preemption_context,
                    } = deadline_preemption.into_inner();

                    self.state =
                        Some(handler.preemption_state(actor, deadline_preemption_context.into()));

                    crate::kernel::Kernel::get()
                        .interrupt_manager()
                        .notify_local_end_of_interrupt();

                    future_context.waker().wake_by_ref();

                    return Some(Poll::Pending);
                }
                ActorUnprivilegedStageExecutorEvent::Exception => {
                    // error
                    todo!()
                }
            }
        }

        None
    }

    fn handle_receive(
        &mut self,
        context: &mut Context<'_>,
        state: ActorUnprivilegedExecutorReceiveState<A, H>,
    ) -> Option<Poll<()>> {
        let mut actor = state.into_inner().actor;

        let result = {
            let mut pinned = pin!(self.receiver.receive());

            pinned.as_mut().poll(context)
        };

        match result {
            Poll::Pending => {
                self.state = Some(ActorUnprivilegedExecutorReceiveState::new(actor).into());

                return Some(Poll::Pending);
            }
            Poll::Ready(None) => {
                self.state = Some(ActorUnprivilegedExecutorDestroyState::new(actor, None).into());
            }
            Poll::Ready(Some(message)) => {
                self.state = Some(ActorUnprivilegedExecutorHandleState::new(actor, message).into());
            }
        }

        None
    }
}

impl<A, B, H> Future for ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, future_context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.state.take() {
                Some(ActorUnprivilegedExecutorState::Create(state)) => {
                    let ActorUnprivilegedExecutorCreateStateInner {
                        mut actor,
                        context: stage_context,
                        ..
                    } = state.into_inner();

                    let result = self.handle(
                        actor,
                        future_context,
                        stage_context,
                        ActorUnprivilegedExecutorCreateStageHandler,
                    );

                    if let Some(poll) = result {
                        return poll;
                    }
                }
                Some(ActorUnprivilegedExecutorState::Receive(state)) => {
                    if let Some(poll) = self.handle_receive(future_context, state) {
                        return poll;
                    }
                }
                Some(ActorUnprivilegedExecutorState::Handle(state)) => {}
                Some(ActorUnprivilegedExecutorState::Destroy(state)) => {
                    let ActorUnprivilegedExecutorDestroyStateInner {
                        mut actor,
                        context: stage_context,
                        ..
                    } = state.into_inner();

                    let result = self.handle(
                        actor,
                        future_context,
                        stage_context,
                        ActorUnprivilegedExecutorDestroyStageHandler,
                    );

                    if let Some(poll) = result {
                        return poll;
                    }
                }
                None => return Poll::Ready(()),
            }
        }
    }
}
