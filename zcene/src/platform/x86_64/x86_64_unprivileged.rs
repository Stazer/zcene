use crate::platform::unprivileged_event::{
    UnprivilegedEvent, UnprivilegedExceptionEvent, UnprivilegedSystemCallEvent,
};
use core::sync::atomic::AtomicU64;
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

#[unsafe(naked)]
pub unsafe extern "C" fn x86_64_system_call_entry_point() -> ! {
    naked_asm!(

        "mov r9, rcx",
        "mov r10, rsp",

        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",

        "mov rdx, r8",

        "pop rcx",

        "jmp x86_64_unprivileged_system_call_restore",
        "hlt",
    )
}

#[unsafe(naked)]
pub unsafe extern "C" fn x86_64_deadline_preemption_entry_point() {
    naked_asm!(

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

        "mov rdi, rsp",

        "mov rcx, 0xC0000102",
        "rdmsr",
        "shl rdx, 32",
        "or rax, rdx",
        "mov rsp, rax",

        "pop rsi",
        "jmp x86_64_unprivileged_preemption_restore",
        "hlt",
    )
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub static X86_64_KERNEL_EXCEPTION_HANDLERS: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];

macro_rules! exception_entry_point {
    ($name:ident, $vector:expr, error_code) => {
        #[unsafe(naked)]
        pub unsafe extern "C" fn $name() {
            naked_asm!(

                "test qword ptr [rsp + 16], 3",
                "jz 2f",
                "pop r9",
                "mov r10, [rsp]",
                "mov r8, cr2",
                "mov r11d, {vector}",
                "mov ecx, 0xC0000102",
                "rdmsr",
                "shl rdx, 32",
                "or rax, rdx",
                "mov rsp, rax",
                "pop rcx",
                "mov rdi, r11",
                "mov rsi, r9",
                "mov rdx, r10",
                "jmp x86_64_unprivileged_exception_restore",
                "hlt",
                "2:",
                "jmp qword ptr [rip + {table} + {offset}]",
                vector = const $vector,
                table = sym X86_64_KERNEL_EXCEPTION_HANDLERS,
                offset = const ($vector * 8),
            )
        }
    };
    ($name:ident, $vector:expr) => {
        #[unsafe(naked)]
        pub unsafe extern "C" fn $name() {
            naked_asm!(

                "test qword ptr [rsp + 8], 3",
                "jz 2f",
                "xor r9d, r9d",
                "mov r10, [rsp]",
                "mov r8, cr2",
                "mov r11d, {vector}",
                "mov ecx, 0xC0000102",
                "rdmsr",
                "shl rdx, 32",
                "or rax, rdx",
                "mov rsp, rax",
                "pop rcx",
                "mov rdi, r11",
                "mov rsi, r9",
                "mov rdx, r10",
                "jmp x86_64_unprivileged_exception_restore",
                "hlt",
                "2:",
                "jmp qword ptr [rip + {table} + {offset}]",
                vector = const $vector,
                table = sym X86_64_KERNEL_EXCEPTION_HANDLERS,
                offset = const ($vector * 8),
            )
        }
    };
}

exception_entry_point!(x86_64_divide_error_entry_point, 0);
exception_entry_point!(x86_64_invalid_opcode_entry_point, 6);
exception_entry_point!(x86_64_stack_segment_fault_entry_point, 12, error_code);
exception_entry_point!(x86_64_general_protection_fault_entry_point, 13, error_code);
exception_entry_point!(x86_64_page_fault_entry_point, 14, error_code);

////////////////////////////////////////////////////////////////////////////////////////////////////

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

#[unsafe(no_mangle)]
pub extern "C" fn x86_64_unprivileged_exception_restore(
    vector: u64,
    error_code: u64,
    instruction_pointer: u64,
    event: &mut Option<X86_64UnprivilegedEvent>,
    address: u64,
) {
    *event = Some(UnprivilegedEvent::Exception(UnprivilegedExceptionEvent {
        vector,
        error_code,
        instruction_pointer,
        address,
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn x86_64_unprivileged_preemption_restore(
    context: &X86_64UnprivilegedPreemptionContext,
    event: &mut Option<X86_64UnprivilegedEvent>,
) {
    *event = Some(UnprivilegedEvent::Preemption(context.clone()));
}

////////////////////////////////////////////////////////////////////////////////////////////////////

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

            stack = in(reg) stack - 8,
            entry = in(reg) entry,
            in("r11") RFlags::FLAGS_IF.bits(),
            out("rax") _,
            out("rdx") _,
            out("rcx") _,
            clobber_abi("C"),
        )
    }
}

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
