use crate::actor::{
    ActorUnprivilegedExecutorCreateState, ActorUnprivilegedExecutorCreateStateInner,
    ActorUnprivilegedExecutorDestroyState, ActorUnprivilegedExecutorHandleState,
    ActorUnprivilegedExecutorReceiveState, ActorUnprivilegedExecutorState,
    ActorUnprivilegedStageExecutorContext, ActorUnprivilegedStageExecutorEvent,
    ActorUnprivilegedStageExecutorSystemCall, ActorUnprivilegedStageExecutorSystemCallInner,
    ActorUnprivilegedStageExecutorSystemCallType,
};
use crate::kernel::logger::println;
use alloc::boxed::Box;
use core::arch::{asm, naked_asm};
use core::future::Future;
use core::marker::PhantomData;
use core::mem::replace;
use core::num::NonZero;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use pin_project::pin_project;
use zcene_core::actor::{
    Actor, ActorContextBuilder, ActorCreateError, ActorHandler, ActorMessageChannelReceiver,
};
use ztd::{Constructor, From};

use x86::current::registers::rsp;

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
    fn perform_stage(
        &mut self,
        context: &mut Context<'_>,
        mut actor: Box<A>,
        event: &mut ActorUnprivilegedStageExecutorEvent,
    ) -> Poll<()> {
        loop {
            match replace(event, ActorUnprivilegedStageExecutorEvent::None) {
                ActorUnprivilegedStageExecutorEvent::None => {
                    unreachable!()
                }
                ActorUnprivilegedStageExecutorEvent::SystemCall(system_call) => {
                    let ActorUnprivilegedStageExecutorSystemCallInner {
                        r#type,
                        context: system_call_context,
                    } = system_call.into_inner();

                    match r#type {
                        ActorUnprivilegedStageExecutorSystemCallType::Continue => unsafe {
                            Self::reenter(
                                &mut actor,
                                event,
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

                            return Poll::Pending;
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(Poll::Pending) => {
                            return Poll::Pending;
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Poll(Poll::Ready(())) => {
                            break
                        }
                        ActorUnprivilegedStageExecutorSystemCallType::Unknown(_) => {
                            todo!()
                        }
                    }
                }
                ActorUnprivilegedStageExecutorEvent::DeadlinePreemption => {
                    todo!()
                }
                ActorUnprivilegedStageExecutorEvent::Exception => {
                    todo!()
                }
            }
        }

        Poll::Ready(())
    }
}

impl<A, B, H> Future for ActorUnprivilegedExecutor<A, B, H>
where
    A: Actor<H>,
    B: ActorContextBuilder<A, H>,
    H: ActorHandler<CreateContext = (), DestroyContext = ()>,
{
    type Output = ();

    // kernel 0000010000008a68
    // user 3638000
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.state.take() {
                Some(ActorUnprivilegedExecutorState::Create(state)) => {
                    let ActorUnprivilegedExecutorCreateStateInner {
                        mut actor,
                        context: stage_context,
                        ..
                    } = state.into_inner();
                    let mut event = ActorUnprivilegedStageExecutorEvent::None;

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

                            /*match self.perform_stage(context, actor, &mut event) {
                                Some(Poll::Pending) => {
                                    return Poll::Pending;
                                }
                                Some(Poll::Ready(())) => {
                                    todo!()
                                }
                                None => {

                                },
                            }*/
                        }
                        Some(ActorUnprivilegedStageExecutorContext::SystemCall(
                            system_call_context,
                        )) => {
                            unsafe {
                                Self::reenter(
                                    &mut actor,
                                    &mut event,
                                    system_call_context.rsp(),
                                    system_call_context.rip(),
                                    system_call_context.rflags(),
                                );
                            }

                            /*match self.perform_stage(context, actor, &mut event) {
                                Some(Poll::Pending) => {
                                    return Poll::Pending;
                                }
                                Some(Poll::Ready(())) => {
                                    self.state =
                                    todo!()
                                }
                                None => {

                                },
                            }*/
                        }
                        Some(ActorUnprivilegedStageExecutorContext::DeadlinePreemption(_)) => {
                            todo!()
                        }
                    }
                }
                Some(ActorUnprivilegedExecutorState::Receive(state)) => {
                    let mut actor = state.into_inner().actor;

                    let result = {
                        let mut pinned = pin!(self.receiver.receive());

                        pinned.as_mut().poll(context)
                    };

                    match result {
                        Poll::Pending => {
                            self.state =
                                Some(ActorUnprivilegedExecutorReceiveState::new(actor).into());

                            return Poll::Pending;
                        }
                        Poll::Ready(None) => {
                            self.state =
                                Some(ActorUnprivilegedExecutorDestroyState::new(actor).into());
                        }
                        Poll::Ready(Some(message)) => {
                            self.state = Some(
                                ActorUnprivilegedExecutorHandleState::new(actor, message).into(),
                            );
                        }
                    }
                }
                Some(ActorUnprivilegedExecutorState::Handle(state)) => {}
                Some(ActorUnprivilegedExecutorState::Destroy(state)) => {}
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

    extern "C" fn destroy_main(actor: A) -> ! {
        let mut context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.destroy(()));

        let result = match pinned.as_mut().poll(&mut context) {
            Poll::Pending => 0,
            Poll::Ready(result) => 0,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    #[naked]
    unsafe extern "C" fn enter(
        actor: &mut A,
        event: &mut ActorUnprivilegedStageExecutorEvent,
        stack: u64,
        main: extern "C" fn(&mut A) -> !,
    ) {
        naked_asm!(
            //
            // Save event address to kernel stack
            //
            "push rsi",
            //
            // Save callee-saved registers to kernel stack
            //
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            //
            // Temporarily save current kernel stack
            //
            "mov rax, rsp",
            //
            // Create interrupt frame for returning into unprivileged mode
            //
            "push 32 | 3",
            "push rdx",
            "push 0x000",
            "push 24 | 3",
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
        )
    }

    #[naked]
    unsafe extern "C" fn reenter(
        actor: &mut A,
        event: &mut ActorUnprivilegedStageExecutorEvent,
        stack: u64,
        rip: u64,
        rflags: u64,
    ) {
        naked_asm!(
            //
            // Save event address to kernel stack
            //
            "push rsi",
            //
            // Save callee-saved registers to kernel stack
            //
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
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
            // Restore callee-saved registers from user stack
            //
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            //
            // Perform return
            //
            "sysretq",
        )
    }
}
