use crate::actor::{
    ActorIsolationEnvironment,
    ActorIsolationExecutorDeadlinePreemptionContext, ActorIsolationExecutorDeadlinePreemptionEvent, ActorIsolationExecutorEvent, ActorIsolationExecutorSystemCallContext,
    ActorIsolationExecutorSystemCallEvent, ActorIsolationExecutorSystemCallEventInner,
    ActorIsolationExecutorSystemCallType, ActorIsolationMessageHandler, ActorRootEnvironment,
};
use alloc::boxed::Box;
use zcene_bare::memory::allocator::LeakingHeapMemoryAllocator;
use alloc::vec::Vec;
use core::arch::asm;
use core::arch::naked_asm;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZero;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use x86::current::rflags::RFlags;
use zcene_bare::common::As;
use zcene_core::actor::{
    Actor, ActorCommonHandleContext, ActorEnvironmentAllocator, ActorMessageChannelReceiver,
};
use zcene_core::future::r#yield;
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

macro_rules! save_kernel_stack {
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
pub struct ActorIsolationExecutor<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>, Message = AI::Message>,
    H: FutureRuntimeHandler,
{
    allocator: <ActorRootEnvironment<H> as ActorEnvironmentAllocator>::Allocator,
    actor: Box<AI>,
    receiver: ActorMessageChannelReceiver<AR::Message>,
    deadline_in_milliseconds: Option<NonZero<usize>>,
    message_handlers: Vec<
        Box<
            dyn ActorIsolationMessageHandler<ActorRootEnvironment<H>>,
            <ActorRootEnvironment<H> as ActorEnvironmentAllocator>::Allocator,
        >,
        <ActorRootEnvironment<H> as ActorEnvironmentAllocator>::Allocator,
    >,
    #[Constructor(default)]
    marker: PhantomData<(AR, H)>,
}

impl<AI, AR, H> ActorIsolationExecutor<AI, AR, H>
where
    AI: Actor<ActorIsolationEnvironment>,
    AR: Actor<ActorRootEnvironment<H>, Message = AI::Message>,
    H: FutureRuntimeHandler,
{
    pub async fn run(mut self) {
        self.handle(|actor, event, stack| {
            Self::execute(Box::as_mut_ptr(actor), event, stack, Self::create_main);
        })
        .await;

        loop {
            let message = match self.receiver.receive().await {
                None => break,
                Some(message) => message,
            };

            self.handle(move |actor, event, stack| {
                Self::execute2(
                    Box::as_mut_ptr(actor),
                    message,
                    event,
                    stack,
                    Self::handle_main,
                );
            })
            .await;
        }

        self.handle(|actor, event, stack| {
            Self::execute(Box::as_mut_ptr(actor), event, stack, Self::destroy_main);
        })
        .await;
    }

    extern "C" fn create_main(actor: *mut AI) -> ! {
        let mut actor = unsafe { Box::from_raw_in(actor, LeakingHeapMemoryAllocator) };

        let mut future_context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.create(()));

        // TODO: handle result
        let _result = match pinned.as_mut().poll(&mut future_context) {
            Poll::Pending => todo!(),
            Poll::Ready(result) => result,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    extern "C" fn handle_main(actor: *mut AI, message: &AI::Message) -> ! {
        let mut actor = unsafe { Box::from_raw_in(actor, LeakingHeapMemoryAllocator) };

        let mut future_context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.handle(ActorCommonHandleContext::new(message.clone())));

        // TODO: handle result
        let _result = match pinned.as_mut().poll(&mut future_context) {
            Poll::Pending => todo!(),
            Poll::Ready(result) => result,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    extern "C" fn destroy_main(actor: *mut AI) -> ! {
        let actor = unsafe { Box::from_raw_in(actor, LeakingHeapMemoryAllocator) };

        let mut future_context = Context::from_waker(Waker::noop());
        let mut pinned = pin!(actor.destroy(()));

        // TODO: handle result
        let _result = match pinned.as_mut().poll(&mut future_context) {
            Poll::Pending => todo!(),
            Poll::Ready(result) => result,
        };

        unsafe { asm!("mov rdi, 0x2", "syscall", options(noreturn)) }
    }

    fn enable_deadline(&mut self) {
        if let Some(deadline_in_milliseconds) = self.deadline_in_milliseconds {
            crate::kernel::Kernel::get()
                .interrupt_manager()
                .reset_oneshot(Duration::from_millis(
                    usize::from(deadline_in_milliseconds).r#as(),
                ));
        }
    }

    #[inline(never)]
    async fn handle<F>(&mut self, execute_function: F)
    where
        F: FnOnce(&mut Box<AI>, &mut Option<ActorIsolationExecutorEvent>, u64),
    {
        let mut event: Option<ActorIsolationExecutorEvent> = None;

        let user_stack = crate::kernel::Kernel::get()
            .memory_manager()
            .allocate_user_stack()
            .unwrap()
            .initial_memory_address()
            .as_u64();

        self.enable_deadline();

        execute_function(&mut self.actor, &mut event, user_stack);

        loop {
            match event.take() {
                None => break,
                Some(ActorIsolationExecutorEvent::SystemCall(system_call)) => {
                    let ActorIsolationExecutorSystemCallEventInner {
                        r#type,
                        context: system_call_context,
                    } = system_call.into_inner();

                    match r#type {
                        ActorIsolationExecutorSystemCallType::Continue => {
                            Self::continue_from_system_call(&mut event, &system_call_context);
                        }
                        ActorIsolationExecutorSystemCallType::Preempt => {
                            r#yield().await;

                            self.enable_deadline();

                            Self::continue_from_system_call(&mut event, &system_call_context);
                        }
                        ActorIsolationExecutorSystemCallType::Poll(Poll::Pending) => {
                            todo!()
                        }
                        ActorIsolationExecutorSystemCallType::Poll(Poll::Ready(())) => break,
                        ActorIsolationExecutorSystemCallType::SendMessageCopy(mailbox, message) => {
                            match self.message_handlers.get(mailbox) {
                                Some(handler) => {
                                    let _result =
                                        handler.send(&self.allocator, message as *const ()).await;
                                }
                                None => todo!(),
                            }

                            Self::continue_from_system_call(&mut event, &system_call_context);
                        }
                        ActorIsolationExecutorSystemCallType::Unknown(_) => {
                            // error
                            todo!()
                        }
                    }
                }
                Some(ActorIsolationExecutorEvent::DeadlinePreemption(deadline_preemption)) => {
                    crate::kernel::Kernel::get()
                        .interrupt_manager()
                        .notify_local_end_of_interrupt();

                    r#yield().await;

                    self.enable_deadline();

                    Self::continue_from_deadline_preemption(
                        Box::as_mut_ptr(&mut self.actor),
                        &mut event,
                        &deadline_preemption.into_inner().context,
                    );
                }
                Some(ActorIsolationExecutorEvent::Exception) => {
                    todo!()
                }
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
                "push {event}",
                save_kernel_stack!(),
                //
                // Perform return
                //
                "mov rsp, {stack}",
                "mov rcx, {function}",
                "sysretq",
                emergency_halt!(),
                "2:",
                pop_callee_saved_registers!(),
                in("rdi") actor,
                event = in(reg) event,
                stack = in(reg) stack,
                function = in(reg) function,
                in("r11") RFlags::FLAGS_IF.bits(),
                out("rax") _,
                out("rdx") _,
                out("rcx") _,
                clobber_abi("C"),
            )
        }
    }

    #[inline(never)]
    extern "C" fn execute2(
        actor: *mut AI,
        message: AI::Message,
        event: &mut Option<ActorIsolationExecutorEvent>,
        stack: u64,
        function: extern "C" fn(*mut AI, &AI::Message) -> !,
    ) {
        unsafe {
            asm!(
                push_callee_saved_registers!(),
                push_inline_return_address!(),
                "push {event}",
                save_kernel_stack!(),
                //
                // Perform return
                //
                "mov rsp, {stack}",
                "mov rcx, {function}",
                "sysretq",
                emergency_halt!(),
                "2:",
                pop_callee_saved_registers!(),
                in("rdi") actor,
                in("rsi") &message,
                event = in(reg) event,
                stack = in(reg) stack,
                function = in(reg) function,
                in("r11") RFlags::FLAGS_IF.bits(),
                out("rax") _,
                out("rdx") _,
                out("rcx") _,
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
                "push rsi",
                save_kernel_stack!(),
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
        event: &mut Option<ActorIsolationExecutorEvent>,
        context: &ActorIsolationExecutorSystemCallContext,
    ) {
        unsafe {
            asm!(
                push_callee_saved_registers!(),
                push_inline_return_address!(),
                "push {event}",
                save_kernel_stack!(),
                //
                // Perform return
                //
                "mov rsp, {stack}",
                "mov rcx, {function}",
                "sysretq",
                emergency_halt!(),
                "2:",
                pop_callee_saved_registers!(),
                event = in(reg) event,
                stack = in(reg) context.rsp(),
                function = in(reg) context.rip(),
                in("r11") context.rflags(),
                out("rax") _,
                out("rdx") _,
                out("rcx") _,
                clobber_abi("C"),
            )
        }
    }
}

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

#[naked]
pub unsafe extern "C" fn actor_system_call_entry_point() -> ! {
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
                3 => ActorIsolationExecutorSystemCallType::SendMessageCopy(
                    argument0 as _,
                    argument1 as _,
                ),
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
    unsafe { asm!("hlt", options(noreturn),) }
}
