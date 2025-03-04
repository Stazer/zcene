use crate::actor::{
    ActorIsolationEnvironment, ActorIsolationExecutorContext,
    ActorIsolationExecutorDeadlinePreemptionContext, ActorIsolationExecutorDeadlinePreemptionEvent,
    ActorIsolationExecutorDeadlinePreemptionEventInner, ActorIsolationExecutorDestroyState,
    ActorIsolationExecutorEvent, ActorIsolationExecutorHandleState,
    ActorIsolationExecutorReceiveState, ActorIsolationExecutorState,
    ActorIsolationExecutorStateHandler, ActorIsolationExecutorSystemCallContext,
    ActorIsolationExecutorSystemCallEvent, ActorIsolationExecutorSystemCallEventInner,
    ActorIsolationExecutorSystemCallType, ActorRootEnvironment,
    ActorIsolationExecutorResult,
};
use alloc::boxed::Box;
use core::arch::asm;
use core::future::Future;
use core::marker::PhantomData;
use core::mem::replace;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};
use core::time::Duration;
use pin_project::pin_project;
use zcene_bare::common::As;
use zcene_core::actor::{
    Actor, ActorEnvironment, ActorMessageChannel, ActorMessageChannelReceiver,
};
use zcene_core::future::runtime::FutureRuntimeHandler;
use ztd::Constructor;

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

#[derive(Constructor)]
#[pin_project]
pub struct ActorIsolationExecutor<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    state: Option<ActorIsolationExecutorState<AI, AR, H>>,
    receiver: ActorMessageChannelReceiver<AR::Message>,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    #[Constructor(default)]
    marker: PhantomData<(AR, H)>,
}

impl<AI, AR, H> Future for ActorIsolationExecutor<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, future_context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let result = match self.state.take() {
                Some(ActorIsolationExecutorState::Create(state)) => {
                    self.handle(future_context, state)
                }
                Some(ActorIsolationExecutorState::Receive(state)) => {
                    self.handle_receive(future_context, state)
                }
                Some(ActorIsolationExecutorState::Handle(state)) => {
                    ActorIsolationExecutorResult::Next
                }
                Some(ActorIsolationExecutorState::Destroy(state)) => {
                    self.handle(future_context, state)
                }
                None => break Poll::Ready(()),
            };

            if matches!(result, ActorIsolationExecutorResult::Pending) {
                break Poll::Pending;
            }
        }
    }
}

impl<AI, AR, H> ActorIsolationExecutor<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>>,
    H: FutureRuntimeHandler,
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
        future_context: &mut Context<'_>,
        mut state: S,
    ) -> ActorIsolationExecutorResult
    where
        S: ActorIsolationExecutorStateHandler<AI, AR, H>,
    {
        self.enable_deadline();

        let mut event: Option<ActorIsolationExecutorEvent> = None;

        let actor = Box::as_mut_ptr(state.actor_mut());

        match state.context() {
            None => {
                let user_stack = crate::kernel::Kernel::get()
                    .memory_manager()
                    .allocate_user_stack()
                    .unwrap()
                    .initial_memory_address()
                    .as_u64();

                Self::execute(actor, &mut event, user_stack, S::main);
            }
            Some(ActorIsolationExecutorContext::SystemCall(ref system_call_context)) => {
                Self::continue_from_system_call(actor, &mut event, &system_call_context);
            }
            Some(ActorIsolationExecutorContext::DeadlinePreemption(
                deadline_preemption_context,
            )) => {
                Self::continue_from_deadline_preemption(
                    actor,
                    &mut event,
                    &deadline_preemption_context,
                );
            }
        }

        loop {
            crate::kernel::logger::println!("{:X?}", event);

            match replace(&mut event, None) {
                None => break ActorIsolationExecutorResult::Next,
                Some(ActorIsolationExecutorEvent::SystemCall(system_call)) => {
                    let ActorIsolationExecutorSystemCallEventInner {
                        r#type,
                        context: system_call_context,
                    } = system_call.into_inner();

                    match r#type {
                        ActorIsolationExecutorSystemCallType::Continue => {
                            Self::continue_from_system_call(
                                actor,
                                &mut event,
                                &system_call_context,
                            );
                        }
                        ActorIsolationExecutorSystemCallType::Preempt => {
                            self.state = Some(state.preemption_state(system_call_context.into()));

                            future_context.waker().wake_by_ref();

                            break ActorIsolationExecutorResult::Pending;
                        }
                        ActorIsolationExecutorSystemCallType::Poll(Poll::Pending) => {
                            todo!()
                        }
                        ActorIsolationExecutorSystemCallType::Poll(Poll::Ready(())) => {
                            self.state = state.next_state();

                            break ActorIsolationExecutorResult::Next;
                        }
                        ActorIsolationExecutorSystemCallType::SendMessageCopy(mailbox, message) => {
                            Self::continue_from_system_call(
                                actor,
                                &mut event,
                                &system_call_context,
                            );
                        },
                        ActorIsolationExecutorSystemCallType::Unknown(_) => {
                            // error
                            todo!()
                        }
                    }
                }
                Some(ActorIsolationExecutorEvent::DeadlinePreemption(deadline_preemption)) => {
                    let ActorIsolationExecutorDeadlinePreemptionEventInner {
                        context: deadline_preemption_context,
                    } = deadline_preemption.into_inner();

                    self.state = Some(state.preemption_state(deadline_preemption_context.into()));

                    future_context.waker().wake_by_ref();

                    crate::kernel::Kernel::get()
                        .interrupt_manager()
                        .notify_local_end_of_interrupt();

                    break ActorIsolationExecutorResult::Pending;
                }
                Some(ActorIsolationExecutorEvent::Exception) => {
                    todo!()
                }
            }
        }
    }

    fn handle_receive(
        &mut self,
        future_context: &mut Context<'_>,
        state: ActorIsolationExecutorReceiveState<AI, AR, H>,
    ) -> ActorIsolationExecutorResult {
        let result = {
            let mut pinned = pin!(self.receiver.receive());

            pinned.as_mut().poll(future_context)
        };

        let actor = state.into_inner().actor;

        match result {
            Poll::Pending => {
                self.state = Some(ActorIsolationExecutorReceiveState::new(actor).into());
                ActorIsolationExecutorResult::Pending
            }
            Poll::Ready(None) => {
                self.state = Some(ActorIsolationExecutorDestroyState::new(actor, None).into());
                ActorIsolationExecutorResult::Next
            }
            Poll::Ready(Some(message)) => {
                self.state = Some(ActorIsolationExecutorHandleState::new(actor, message).into());
                ActorIsolationExecutorResult::Next
            }
        }
    }

    #[inline(never)]
    extern "C" fn execute(
        actor: *mut AI,
        event: &mut Option<ActorIsolationExecutorEvent>,
        stack: u64,
        function: extern "C" fn(*mut AI) -> !,
    ) {
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
    extern "C" fn continue_from_deadline_preemption(
        actor: *mut AI,
        event: &mut Option<ActorIsolationExecutorEvent>,
        context: &ActorIsolationExecutorDeadlinePreemptionContext,
    ) {
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
    extern "C" fn continue_from_system_call(
        actor: *mut AI,
        event: &mut Option<ActorIsolationExecutorEvent>,
        context: &ActorIsolationExecutorSystemCallContext,
    ) {
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
                in("rdi") actor,
                in("rsi") event,
                in("rdx") context.rsp(),
                in("rcx") context.rip(),
                in("r8") context.rflags(),
                clobber_abi("C"),
            )
        }
    }
}

use core::arch::naked_asm;

#[naked]
pub unsafe extern "C" fn actor_deadline_preemption_entry_point() {
    naked_asm!(
        //
        // Save user context
        //
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
        //
        // Save interrupt stack
        //
        "mov rdi, rsp",
        //
        // Load kernel stack
        //
        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",
        //
        // Perform restore
        //
        "pop rsi",
        "call actor_deadline_preemption_restore",
        "ret",
        //
        // Emergency halt
        //
        "hlt",
    )
}

#[no_mangle]
pub extern "C" fn actor_deadline_preemption_restore(
    context: &mut ActorIsolationExecutorDeadlinePreemptionContext,
    event: &mut Option<ActorIsolationExecutorEvent>,
) {
    *event = Some(ActorIsolationExecutorDeadlinePreemptionEvent::new(context.clone()).into());
}


/*#[no_mangle]
#[inline(never)]
pub extern "C" fn actor_system_call_entry_point() -> ! {
    unsafe {
        asm!(
            // rdi = system call identifier
            // rsi = argument 0
            //  r8 = argument 1

            // rcx = instruction address
            // r11 = flags

            //
            // Store user context
            //
            "mov r9, rcx",
            "mov r10, rsp",

            //
            // Load kernel stack
            //
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",

            "mov rdx, r8",
            "pop rcx",
            "call {}",
            "ret",
            emergency_halt!(),

            sym actor_system_call_restore,
            options(noreturn),
        )
    }
}*/

#[naked]
pub unsafe fn actor_system_call_entry_point() -> ! {
    naked_asm!(
        //
        // Store user context
        //
        "mov r9, rcx",
        "mov r10, rsp",
        //
        // Load kernel stack
        //
        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",
        //
        // Restore
        //
        "mov rdx, r8",
        "pop rcx",
        "call actor_system_call_restore",
        "ret",
        emergency_halt!(),
    )

}

#[no_mangle]
#[inline(never)]
pub extern "C" fn actor_system_call_restore(
    system_call_identifier: usize,
    argument0: u64,
    argument1: u64,
    event: &mut Option<ActorIsolationExecutorEvent>,
) {
    let rip: u64;
    let rsp: u64;
    let rflags: u64;

    unsafe { 
        asm!(
            "mov {}, r9",
            "mov {}, r10",
            "mov {}, r11",
            out(reg) rip,
            out(reg) rsp,
            out(reg) rflags,
        );
    }

    *event = Some(
        ActorIsolationExecutorSystemCallEvent::new(
            match system_call_identifier {
                0 => ActorIsolationExecutorSystemCallType::Continue,
                1 => ActorIsolationExecutorSystemCallType::Preempt,
                2 => ActorIsolationExecutorSystemCallType::Poll(Poll::Ready(())),
                3 => {
                    ActorIsolationExecutorSystemCallType::SendMessageCopy(argument0 as _, argument1 as _)
                }
                system_call_identifier => {
                    ActorIsolationExecutorSystemCallType::Unknown(system_call_identifier)
                }
            },
            ActorIsolationExecutorSystemCallContext::new(rsp, rip, rflags),
        )
        .into(),
    );
}

#[inline(never)]
pub extern "C" fn actor_exception_entry_point() -> ! {
    unsafe {
        asm!(
            "hlt",
            options(noreturn),
        )
    }
}
