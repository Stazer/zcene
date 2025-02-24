use crate::actor::{
    ActorUnprivilegedExecutorCreateState, ActorUnprivilegedExecutorCreateStateInner,
    ActorUnprivilegedExecutorDestroyState, ActorUnprivilegedExecutorHandleState,
    ActorUnprivilegedExecutorReceiveState, ActorUnprivilegedExecutorState,
    ActorUnprivilegedStageExecutorContext, ActorUnprivilegedStageExecutorDeadlinePreemptionContext,
    ActorUnprivilegedStageExecutorDeadlinePreemptionInner, ActorUnprivilegedStageExecutorEvent,
    ActorUnprivilegedStageExecutorSystemCall, ActorUnprivilegedStageExecutorSystemCallInner,
    ActorUnprivilegedStageExecutorSystemCallType,
};
use crate::kernel::logger::println;
use alloc::boxed::Box;
use core::arch::{asm, naked_asm};
use core::future::Future;
use core::ptr::NonNull;
use core::marker::PhantomData;
use core::mem::replace;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use pin_project::pin_project;
use zcene_bare::common::As;
use zcene_core::actor::{
    Actor, ActorContextBuilder, ActorCreateError, ActorHandler, ActorMessageChannelReceiver,
};
use ztd::{Constructor, From};

////////////////////////////////////////////////////////////////////////////////////////////////////

/*#[derive(Constructor)]
pub struct ActorUnprivilegedStageExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    context: Option<ActorUnprivilegedStageExecutorContext>,
    #[Constructor(default)]
    marker: PhantomData<H>,
}*/

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
    fn handle_create(
        &mut self,
        context: &mut Context<'_>,
        state: ActorUnprivilegedExecutorCreateState<A, H>,
    ) -> Option<Poll<()>> {
        let ActorUnprivilegedExecutorCreateStateInner {
            mut actor,
            context: stage_context,
            ..
        } = state.into_inner();

        let mut event = ActorUnprivilegedStageExecutorEvent::None;

        if let Some(deadline_in_milliseconds) = self.deadline_in_milliseconds {
            crate::kernel::Kernel::get()
                .interrupt_manager()
                .reset_oneshot(Duration::from_millis(
                    usize::from(deadline_in_milliseconds).r#as(),
                ));
        }

        match stage_context {
            None => {
                let user_stack = crate::kernel::Kernel::get()
                    .memory_manager()
                    .allocate_user_stack()
                    .unwrap()
                    .initial_memory_address()
                    .as_u64();

                unsafe {
                    Self::enter(&mut actor, &mut event, user_stack, Self::create_main);
                }
            }
            Some(ActorUnprivilegedStageExecutorContext::SystemCall(
                system_call_context,
            )) => unsafe {
                Self::system_return(
                    &mut actor,
                    &mut event,
                    system_call_context.rsp(),
                    system_call_context.rip(),
                    system_call_context.rflags(),
                );
            },
            Some(ActorUnprivilegedStageExecutorContext::DeadlinePreemption(
                deadline_preemption_context,
            )) => {
                unsafe {
                    Self::r#continue(&mut actor, &mut event, &deadline_preemption_context);
                }
            },
        }

        loop {
            match replace(&mut event, ActorUnprivilegedStageExecutorEvent::None) {
                ActorUnprivilegedStageExecutorEvent::None => break,
                ActorUnprivilegedStageExecutorEvent::SystemCall(system_call) => {
                    let ActorUnprivilegedStageExecutorSystemCallInner {
                        r#type,
                        context: system_call_context,
                    } = system_call.into_inner();

                    match r#type {
                        ActorUnprivilegedStageExecutorSystemCallType::Continue => unsafe {
                            Self::system_return(
                                &mut actor,
                                &mut event,
                                system_call_context.rsp(),
                                system_call_context.rip(),
                                system_call_context.rflags(),
                            );
                        },
                        ActorUnprivilegedStageExecutorSystemCallType::Preempt => {
                            self.state = Some(
                                ActorUnprivilegedExecutorCreateState::new(
                                    actor,
                                    Some(system_call_context.into()),
                                )
                                    .into(),
                            );

                            context.waker().wake_by_ref();

                            return Some(Poll::Pending);
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(
                            Poll::Pending,
                        ) => {
                            todo!()
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(
                            Poll::Ready(()),
                        ) => {
                            self.state = Some(
                                ActorUnprivilegedExecutorReceiveState::new(actor)
                                    .into(),
                            );
                            break;
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Unknown(_) => {
                            // error
                            todo!()
                        }
                    }
                }
                ActorUnprivilegedStageExecutorEvent::DeadlinePreemption(
                    deadline_preemption,
                ) => {
                    let ActorUnprivilegedStageExecutorDeadlinePreemptionInner {
                        context: deadline_preemption_context,
                    } = deadline_preemption.into_inner();

                    self.state = Some(
                        ActorUnprivilegedExecutorCreateState::new(
                            actor,
                            Some(deadline_preemption_context.into()),
                        )
                            .into(),
                    );

                    crate::kernel::Kernel::get()
                        .interrupt_manager()
                        .notify_local_end_of_interrupt();

                    context.waker().wake_by_ref();

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
                self.state =
                    Some(ActorUnprivilegedExecutorReceiveState::new(actor).into());

                return Some(Poll::Pending);
            }
            Poll::Ready(None) => {
                self.state =
                    Some(ActorUnprivilegedExecutorDestroyState::new(actor, None).into());
            }
            Poll::Ready(Some(message)) => {
                self.state = Some(
                    ActorUnprivilegedExecutorHandleState::new(actor, message).into(),
                );
            }
        }

        None
    }

    fn handle_destroy(
        &mut self,
        context: &mut Context<'_>,
        state: ActorUnprivilegedExecutorDestroyState<A, H>,
    ) -> Option<Poll<()>> {
        let ActorUnprivilegedExecutorDestroyStateInner {
            mut actor,
            context: stage_context,
            ..
        } = state.into_inner();

        let mut event = ActorUnprivilegedStageExecutorEvent::None;

        if let Some(deadline_in_milliseconds) = self.deadline_in_milliseconds {
            crate::kernel::Kernel::get()
                .interrupt_manager()
                .reset_oneshot(Duration::from_millis(
                    usize::from(deadline_in_milliseconds).r#as(),
                ));
        }

        match stage_context {
            None => {
                let user_stack = crate::kernel::Kernel::get()
                    .memory_manager()
                    .allocate_user_stack()
                    .unwrap()
                    .initial_memory_address()
                    .as_u64();

                unsafe {
                    Self::enter(&mut actor, &mut event, user_stack, Self::destroy_main);
                }
            }
            Some(ActorUnprivilegedStageExecutorContext::SystemCall(
                system_call_context,
            )) => unsafe {
                Self::system_return(
                    &mut actor,
                    &mut event,
                    system_call_context.rsp(),
                    system_call_context.rip(),
                    system_call_context.rflags(),
                );
            },
            Some(ActorUnprivilegedStageExecutorContext::DeadlinePreemption(
                deadline_preemption_context,
            )) => {
                unsafe {
                    Self::r#continue(&mut actor, &mut event, &deadline_preemption_context);
                }
            },
        }

        loop {
            match replace(&mut event, ActorUnprivilegedStageExecutorEvent::None) {
                ActorUnprivilegedStageExecutorEvent::None => break,
                ActorUnprivilegedStageExecutorEvent::SystemCall(system_call) => {
                    let ActorUnprivilegedStageExecutorSystemCallInner {
                        r#type,
                        context: system_call_context,
                    } = system_call.into_inner();

                    match r#type {
                        ActorUnprivilegedStageExecutorSystemCallType::Continue => unsafe {
                            Self::system_return(
                                &mut actor,
                                &mut event,
                                system_call_context.rsp(),
                                system_call_context.rip(),
                                system_call_context.rflags(),
                            );
                        },
                        ActorUnprivilegedStageExecutorSystemCallType::Preempt => {
                            self.state = Some(
                                ActorUnprivilegedExecutorCreateState::new(
                                    actor,
                                    Some(system_call_context.into()),
                                )
                                    .into(),
                            );

                            context.waker().wake_by_ref();

                            return Some(Poll::Pending);
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(
                            Poll::Pending,
                        ) => {
                            todo!()
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(
                            Poll::Ready(()),
                        ) => {
                            self.state = Some(
                                ActorUnprivilegedExecutorReceiveState::new(actor)
                                    .into(),
                            );
                            break;
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Unknown(_) => {
                            // error
                            todo!()
                        }
                    }
                }
                ActorUnprivilegedStageExecutorEvent::DeadlinePreemption(
                    deadline_preemption,
                ) => {
                    let ActorUnprivilegedStageExecutorDeadlinePreemptionInner {
                        context: deadline_preemption_context,
                    } = deadline_preemption.into_inner();

                    self.state = Some(
                        ActorUnprivilegedExecutorCreateState::new(
                            actor,
                            Some(deadline_preemption_context.into()),
                        )
                            .into(),
                    );

                    crate::kernel::Kernel::get()
                        .interrupt_manager()
                        .notify_local_end_of_interrupt();

                    context.waker().wake_by_ref();

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
}

impl<A, B, H> Future for ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.state.take() {
                Some(ActorUnprivilegedExecutorState::Create(state)) => {
                    if let Some(poll) = self.handle_create(
                        context,
                        state,
                    ) {
                        return poll;
                    }
                }
                Some(ActorUnprivilegedExecutorState::Receive(state)) => {
                    if let Some(poll) = self.handle_receive(context, state) {
                        return poll;
                    }
                }
                Some(ActorUnprivilegedExecutorState::Handle(state)) => {
                }
                Some(ActorUnprivilegedExecutorState::Destroy(state)) => {
                }
                None => return Poll::Ready(()),
            }
        }
    }
}

impl<A, B, H> ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    extern "C" fn create_main(actor: &mut A) -> ! {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn, nostack)) }
    }

    extern "C" fn handle_main(actor: &mut A) -> ! {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.create(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    extern "C" fn destroy_main(actor: Box<A>) -> ! {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.destroy(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    #[no_mangle]
    #[inline(never)]
    unsafe extern "C" fn enter(
        actor: &mut A,
        mut event: &mut ActorUnprivilegedStageExecutorEvent,
        stack: u64,
        main: extern "C" fn(&mut A) -> !,
    ) {
        asm!(
            //
            // Save callee-saved registers to kernel stack
            //
            "push rbx",
            "push rbp",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            //
            // Save inline return address
            //
            "lea rax, [2f]",
            "push rax",
            //
            // Save event address to kernel stack
            //
            "push rsi",
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
            //
            // Inline label for return
            //
            "2:",
            //
            // Restore callee-saved registers
            //
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbp",
            "pop rbx",
            in("rdi") actor,
            in("rsi") event,
            in("rdx") stack,
            in("rcx") main,
            clobber_abi("C"),
        )
    }

    #[no_mangle]
    #[inline(never)]
    unsafe extern "C" fn system_return(
        actor: &mut A,
        event: &mut ActorUnprivilegedStageExecutorEvent,
        stack: u64,
        rip: u64,
        rflags: u64,
    ) {
        asm!(
            //
            // Save callee-saved registers to kernel stack
            //
            "push rbx",
            "push rbp",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            //
            // Save inline return address
            //
            "lea rax, [2f]",
            "push rax",
            //
            // Save event address to kernel stack
            //
            "push rsi",
            //
            // Temporarily save current kernel stack
            //
            "mov rax, rsp",
            //
            // Move arguments into temporary registers
            //
            "mov r9, rdx",
            "mov r10, rcx",
            //
            // Store kernel stack
            //
            "mov rax, rsp",
            "mov rdx, rsp",
            "shr rdx, 32",
            "mov rcx, 0xC0000102",
            "wrmsr",
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
            //
            // Inline label for return
            //
            "2:",
            //
            // Restore callee-saved registers
            //
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbp",
            "pop rbx",
            in("rdi") actor,
            in("rsi") event,
            in("rdx") stack,
            in("rcx") rip,
            in("r8") rflags,

            clobber_abi("C"),
        )
    }

    #[inline(never)]
    unsafe extern "C" fn r#continue(
        actor: &mut A,
        event: &mut ActorUnprivilegedStageExecutorEvent,
        context: &ActorUnprivilegedStageExecutorDeadlinePreemptionContext,
    ) {
        asm!(
            //
            // Save callee-saved registers to kernel stack
            //
            "push rbx",
            "push rbp",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            //
            // Save inline return address
            //
            "lea rax, [2f]",
            "push rax",
            //
            // Save event address to kernel stack
            //
            "push rsi",
            //
            // Store kernel stack
            //
            "mov rax, rsp",
            "mov rdx, rsp",
            "shr rdx, 32",
            "mov rcx, 0xC0000102",
            "wrmsr",
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
            //
            // Emergency halt
            //
            "hlt",
            //
            // Restore callee-saved registers
            //
            "2:",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbp",
            "pop rbx",
            in("rdi") actor,
            in("rsi") event,
            in("r8") context,
            clobber_abi("C"),
        )
    }
}
