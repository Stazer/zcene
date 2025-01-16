[BITS 64]

section .text

extern timer_interrupt_handler

%macro save_basic_context 0
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

    pushfq
%endmacro

%macro restore_basic_context 0
    popfq

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
timer_interrupt_entry_point:
    jmp timer_interrupt_handler

    iretq
