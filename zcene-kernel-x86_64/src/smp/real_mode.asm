[BITS 16]

extern smp_protected_mode_entry

section .smp_real_mode_section

global smp_real_mode_entry
smp_real_mode_entry:
    ; Disable Interrupts
    cli

    ; GDT
    lgdt [gdtr]

    ; Protected Mode
    mov eax, cr0
    or eax, 0x1
    mov cr0, eax

    jmp 08h:smp_protected_mode_entry

gdtr:
    dw gdt_end - gdt - 1
    dd gdt

gdt:
    dd 0x00000000, 0x00000000

    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 0x9A
    db 0xCF
    db 0x00

    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 0x92
    db 0xCF
    db 0x00

gdt_end:
