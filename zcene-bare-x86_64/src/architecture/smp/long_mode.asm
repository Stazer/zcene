[BITS 64]

section .smp_long_mode_section

extern application_processor_entry_point
extern SMP_HEADER

global smp_long_mode_entry
smp_long_mode_entry:
    ; Disable Interrupts
    cli

    ; Setup Segments
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    ; Fetch Local-APIC ID
    mov rax, 0x1
    cpuid
    shr rbx, 0x18
    mov rax, rbx

    ; Calculate Stack Address
    sub rax, 1
    mov rbx, [SMP_HEADER + 0x8]
    mul rbx
    add rax, [SMP_HEADER]

    ; Setup Stack
    mov rsp, rax

    jmp application_processor_entry_point

    hlt
