mod actor_unprivileged_executor;
mod actor_unprivileged_executor_create_state;
mod actor_unprivileged_executor_destroy_state;
mod actor_unprivileged_executor_handle_state;
mod actor_unprivileged_executor_receive_state;
mod actor_unprivileged_executor_stage_event;
mod actor_unprivileged_executor_state;
mod actor_unprivileged_executor_system_call;

pub use actor_unprivileged_executor::*;
pub use actor_unprivileged_executor_create_state::*;
pub use actor_unprivileged_executor_destroy_state::*;
pub use actor_unprivileged_executor_handle_state::*;
pub use actor_unprivileged_executor_receive_state::*;
pub use actor_unprivileged_executor_stage_event::*;
pub use actor_unprivileged_executor_state::*;
pub use actor_unprivileged_executor_system_call::*;

use core::arch::naked_asm;

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
        //"jmp actor_deadline_restore",
    )
}

#[naked]
pub unsafe extern "C" fn actor_system_call_entry_point() {
    naked_asm!(
        // Load kernel stack pointer
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
        // At this stage it looks like we execute from within the future
        "cld",
        "ret",
    )
}

#[naked]
pub unsafe extern "C" fn actor_exception_entry_point() {
    naked_asm!(
        // Load kernel stack pointer
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
        // At this stage it looks like we execute from within the future
        "cld",
        "ret",
    )
}
