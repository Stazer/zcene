use crate::platform::unprivileged_event::{UnprivilegedEvent, UnprivilegedSystemCallEvent};
use crate::platform::x86_64::{
    X86_64UnprivilegedPreemptionContext, X86_64UnprivilegedSystemCallContext,
};
use core::arch::{asm, naked_asm};
use x86::current::rflags::RFlags;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub type X86_64UnprivilegedEvent = UnprivilegedEvent<X86_64UnprivilegedSystemCallContext, X86_64UnprivilegedPreemptionContext>;

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

////////////////////////////////////////////////////////////////////////////////////////////////////

/// System call entry point registered via IA32_LSTAR MSR.
///
/// On syscall: rcx = user rip, r11 = user rflags, rsp = user rsp.
/// r8 = second argument (message pointer for SendMessageCopy).
/// The kernel stack (saved in KernelGSBase) holds: [event_ptr, return_addr, callee_saved_regs].
#[unsafe(naked)]
pub unsafe extern "C" fn x86_64_system_call_entry_point() -> ! {
    unsafe {
        naked_asm!(
            // Save user context registers that syscall doesn't preserve
            "mov r9, rcx",   // user rip (rcx clobbered by syscall)
            "mov r10, rsp",  // user rsp
            // Load kernel stack from KernelGSBase MSR
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",
            // Translate r8 (message pointer) into rdx for the restore fn (arg2)
            "mov rdx, r8",
            // Pop event pointer (was pushed before sysretq in execute_in_unprivileged)
            "pop rcx",
            // Tail-call restore; on return, execution continues at the saved return address
            "jmp x86_64_unprivileged_system_call_restore",
            "hlt",
        )
    }
}

/// Deadline preemption interrupt handler (x86-interrupt ABI, uses IST slot 0).
///
/// Saves the full user register file onto the interrupt stack, then switches to the
/// kernel stack and calls the preemption restore function.
#[unsafe(naked)]
pub unsafe extern "C" fn x86_64_deadline_preemption_entry_point() {
    unsafe {
        naked_asm!(
            // Save all GP registers (CPU has already pushed rip/cs/rflags/rsp/ss below these)
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
            // rdi = pointer to saved context (X86_64UnprivilegedPreemptionContext)
            "mov rdi, rsp",
            // Load kernel stack from KernelGSBase MSR
            "mov rcx, 0xC0000102",
            "rdmsr",
            "shl rdx, 32",
            "or rax, rdx",
            "mov rsp, rax",
            // Pop event pointer into rsi (second arg to restore fn)
            "pop rsi",
            "jmp x86_64_unprivileged_preemption_restore",
            "hlt",
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Called (via tail-call jmp) from `x86_64_system_call_entry_point`.
///
/// Arguments follow the C ABI after the entry point's register setup:
/// - rdi = syscall identifier
/// - rsi = argument0
/// - rdx = argument1 (message pointer for SendMessageCopy)
/// - rcx = &mut Option<X86_64UnprivilegedEvent>
///
/// r9/r10/r11 still hold user rip/rsp/rflags at the time of the call.
#[unsafe(no_mangle)]
pub extern "C" fn x86_64_unprivileged_system_call_restore(
    identifier: usize,
    argument0: u64,
    argument1: u64,
    event: &mut Option<X86_64UnprivilegedEvent>,
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

    *event = Some(UnprivilegedEvent::SystemCall(UnprivilegedSystemCallEvent {
        identifier,
        argument0,
        argument1,
        context: X86_64UnprivilegedSystemCallContext::new(rsp, rip, rflags),
    }));
}

/// Called (via tail-call jmp) from `x86_64_deadline_preemption_entry_point`.
///
/// - rdi = *mut X86_64UnprivilegedPreemptionContext (points into the interrupt stack)
/// - rsi = &mut Option<X86_64UnprivilegedEvent>
#[unsafe(no_mangle)]
pub extern "C" fn x86_64_unprivileged_preemption_restore(
    context: &X86_64UnprivilegedPreemptionContext,
    event: &mut Option<X86_64UnprivilegedEvent>,
) {
    *event = Some(UnprivilegedEvent::Preemption(context.clone()));
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// Switches to unprivileged mode (ring 3) and executes `entry(argument, message)`.
///
/// The kernel stack context is set up so that when the user code finishes via
/// `syscall`, execution resumes at label "2:" and this function returns normally.
///
/// Must be `#[inline(never)]` — the asm manipulates the raw call stack.
#[inline(never)]
pub extern "C" fn x86_64_execute_in_unprivileged(
    argument: *mut (),
    message: *const (),
    event: &mut Option<X86_64UnprivilegedEvent>,
    stack: u64,
    entry: extern "C" fn(*mut (), *const ()) -> !,
) {
    unsafe {
        asm!(
            push_callee_saved_registers!(),
            push_inline_return_address!(),
            "push {event}",
            save_kernel_stack!(),
            "mov rsp, {stack}",
            "mov rcx, {entry}",
            "sysretq",
            "hlt",
            "2:",
            pop_callee_saved_registers!(),
            in("rdi") argument,
            in("rsi") message,
            event = in(reg) event,
            stack = in(reg) stack,
            entry = in(reg) entry,
            in("r11") RFlags::FLAGS_IF.bits(),
            out("rax") _,
            out("rdx") _,
            out("rcx") _,
            clobber_abi("C"),
        )
    }
}

/// Resumes unprivileged execution after a system call by returning to the saved user context.
///
/// Must be `#[inline(never)]` — the asm manipulates the raw call stack.
#[inline(never)]
pub extern "C" fn x86_64_resume_after_system_call(
    context: &X86_64UnprivilegedSystemCallContext,
    event: &mut Option<X86_64UnprivilegedEvent>,
) {
    unsafe {
        asm!(
            push_callee_saved_registers!(),
            push_inline_return_address!(),
            "push {event}",
            save_kernel_stack!(),
            "mov rsp, {stack}",
            "mov rcx, {rip}",
            "sysretq",
            "hlt",
            "2:",
            pop_callee_saved_registers!(),
            event = in(reg) event,
            stack = in(reg) context.rsp(),
            rip = in(reg) context.rip(),
            in("r11") context.rflags(),
            out("rax") _,
            out("rdx") _,
            out("rcx") _,
            clobber_abi("C"),
        )
    }
}

/// Resumes unprivileged execution after a deadline preemption via `iretq`.
///
/// Must be `#[inline(never)]` — the asm manipulates the raw call stack.
#[inline(never)]
pub extern "C" fn x86_64_resume_after_preemption(
    context: &X86_64UnprivilegedPreemptionContext,
    event: &mut Option<X86_64UnprivilegedEvent>,
) {
    unsafe {
        asm!(
            push_callee_saved_registers!(),
            push_inline_return_address!(),
            "push {event}",
            save_kernel_stack!(),
            // Switch the stack pointer to the saved context (on the interrupt stack).
            // Popping restores GP registers; iretq restores rip/cs/rflags/rsp/ss.
            "mov rsp, {ctx}",
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
            "hlt",
            "2:",
            pop_callee_saved_registers!(),
            event = in(reg) event,
            ctx = in(reg) context as *const _,
            clobber_abi("C"),
        )
    }
}
