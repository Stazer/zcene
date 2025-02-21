mod actor_unprivileged_executor;
mod actor_unprivileged_executor_create_state;
mod actor_unprivileged_executor_destroy_state;
mod actor_unprivileged_executor_handle_state;
mod actor_unprivileged_executor_receive_state;
mod actor_unprivileged_executor_stage_context;
mod actor_unprivileged_executor_stage_event;
mod actor_unprivileged_executor_state;

pub use actor_unprivileged_executor::*;
pub use actor_unprivileged_executor_create_state::*;
pub use actor_unprivileged_executor_destroy_state::*;
pub use actor_unprivileged_executor_handle_state::*;
pub use actor_unprivileged_executor_receive_state::*;
pub use actor_unprivileged_executor_stage_context::*;
pub use actor_unprivileged_executor_stage_event::*;
pub use actor_unprivileged_executor_state::*;

use crate::kernel::logger::println;
use core::arch::naked_asm;
use core::task::Poll;

#[naked]
pub unsafe extern "C" fn actor_preemption_entry_point() {
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
        // Load kernel stack
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
        // Return into restore
        "cld",
        "ret",
    )
}

#[naked]
pub unsafe extern "C" fn actor_system_call_entry_point() {
    naked_asm!(
        //
        // Save callee-saved registers
        //
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        //
        // Store user stack
        //
        "mov rsi, rsp",
        "mov r8, rcx",
        //
        // Load kernel stack
        //
        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",
        //
        // Restore callee-saved registers
        //
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        //
        // Prepare arguments. First argument (rdi) is passed from the system call itself
        // Prepare second argument
        //
        "mov rdx, r8",
        "mov rcx, r11",
        "pop r8",
        //
        // Perform restore
        //
        "cld",
        "jmp actor_system_call_restore",
    )
}

#[no_mangle]
pub extern "C" fn actor_system_call_restore(
    system_call_number: usize,
    rsp: u64,
    rip: u64,
    rflags: u64,
    event: &mut ActorUnprivilegedStageExecutorEvent,
) {
    *event =
        ActorUnprivilegedStageExecutorEvent::from(ActorUnprivilegedStageExecutorSystemCall::new(
            match system_call_number {
                0 => ActorUnprivilegedStageExecutorSystemCallType::Continue,
                1 => ActorUnprivilegedStageExecutorSystemCallType::Preempt,
                2 => ActorUnprivilegedStageExecutorSystemCallType::Poll(Poll::Ready(())),
                system_call_number => {
                    ActorUnprivilegedStageExecutorSystemCallType::Unknown(system_call_number)
                }
            },
            ActorUnprivilegedStageExecutorSystemCallContext::new(rsp, rip, rflags),
        ));
}

#[naked]
pub unsafe extern "C" fn actor_exception_entry_point() {
    naked_asm!(
        // Load kernel stack
        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",
        // First argument is passed from the system call itself
        // Prepare second argument
        "pop rsi",
        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "ret",
    )
}
