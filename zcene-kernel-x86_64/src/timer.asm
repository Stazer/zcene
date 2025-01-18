[BITS 64]

section .text

extern timer_interrupt_handler
extern timer_interrupt_handler2

%macro save_basic_registers_context 0
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
%endmacro

%macro restore_basic_registers_context 0
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
%endmacro

global timer_interrupt_entry_point
global after_preemption
timer_interrupt_entry_point:
    save_basic_registers_context
    mov rdi, rsp
    call timer_interrupt_handler
after_preemption:
    restore_basic_registers_context
    iretq
