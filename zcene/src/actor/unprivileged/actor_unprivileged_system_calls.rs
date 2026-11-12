use core::arch::asm;

////////////////////////////////////////////////////////////////////////////////////////////////////

/// The kernel does not preserve any register across a system call except the stack pointer, so
/// the callee-saved ones are saved by hand around it.
macro_rules! system_call {
    ($identifier:expr, $argument0:expr, $argument1:expr) => {
        asm!(
            "push rbx",
            "push rbp",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "syscall",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbp",
            "pop rbx",
            in("rdi") $identifier,
            in("rsi") $argument0,
            in("r8") $argument1,
            clobber_abi("C"),
        )
    };
}

/// The system calls an unprivileged actor can make.
pub struct ActorUnprivilegedSystemCalls;

impl ActorUnprivilegedSystemCalls {
    /// Ends the current phase.
    pub fn exit() -> ! {
        unsafe { asm!("mov edi, 2", "syscall", options(noreturn)) }
    }

    /// Lets other tasks run, then continues.
    pub fn yield_now() {
        unsafe { system_call!(1usize, 0usize, 0usize) }
    }

    /// Copies the message behind `message` to the handler at `recipient`.
    pub fn send_message_copy<M>(recipient: usize, message: &M) {
        unsafe { system_call!(3usize, recipient, message as *const M as *const ()) }
    }

    /// Stops the actor with an invalid instruction, which the kernel reports as a fault.
    pub fn fault() -> ! {
        unsafe { asm!("ud2", options(noreturn)) }
    }
}
