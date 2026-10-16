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

    ; CR4: copied wholesale from the bootstrap processor (PAE for long mode, OSFXSR/OSXMMEXCPT
    ; for SSE, and whatever else it runs with) instead of hand-picking bits here — an
    ; application processor resets with CR4 cleared, and anything the BSP needed to run
    ; normally (e.g. the compiler freely emits SSE2, x86-64 ABI baseline), it needs too.
    mov eax, [SMP_HEADER + 0x1E]
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
