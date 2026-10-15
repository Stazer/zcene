[BITS 32]

section .smp_protected_mode_section

extern smp_long_mode_entry
extern SMP_HEADER

align 32

global smp_protected_mode_entry
smp_protected_mode_entry:
    ; Disable Interrupts
    cli

    ; Setup Segments
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; PAE, and enable SSE (OSFXSR, OSXMMEXCPT): the BSP's CR4 was already set up this way by
    ; the bootloader, but application processors start from a clean reset, and the compiler
    ; freely emits SSE2 for ordinary code (it's x86-64 ABI baseline) — without this, the first
    ; SSE instruction after jumping into Rust raises #UD.
    mov eax, cr4
    or eax, 0x20 | 0x200 | 0x400
    mov cr4, eax

    ; Page Table Pointer
    mov edi, [SMP_HEADER + 0x10]
    mov cr3, edi

    ; Enable Long Mode and No-Execute Flag
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x00000300
    or eax, 0x00000800
    wrmsr

    ; Paging
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax

    ; GDT
    lgdt [SMP_HEADER + 0x14]

    jmp 0x08:smp_long_mode_entry
