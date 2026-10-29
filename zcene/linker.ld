SECTIONS {
    . = 0x0;

    .smp_sections : {
        *(.smp_real_mode_section)
        *(.smp_protected_mode_section)
        *(.smp_long_mode_section)
        *(.smp_header_section)
        *(.smp_header_section.*)
    }

    SMP_SECTIONS_START = ADDR(.smp_sections);
    SMP_SECTIONS_END = ADDR(.smp_sections) + SIZEOF(.smp_sections);
    SMP_SECTIONS_SIZE = SIZEOF(.smp_sections);

    . = 1M;

    . = 0x200000;

    .text : ALIGN(0x1000) {
        *(.text)
        *(.text.*)
    }

    .isr : ALIGN(0x1000) {
        *(.isr)
        *(.isr.*)
    }

    /* What unprivileged actors may execute and read; everything else stays kernel-only. */
    USER_EXECUTABLE_START = ADDR(.text);
    USER_EXECUTABLE_END = ADDR(.isr) + SIZEOF(.isr);

    /* Vtables and the like. Written once at link time, but the linker puts them into a writable
       section, so the kernel makes their pages read-only itself (see the memory manager). Own
       section, so no writable data shares their pages. */
    .data_rel_ro : ALIGN(0x1000) {
        *(.data.rel.ro)
        *(.data.rel.ro.*)
    }

    USER_READ_ONLY_START = ADDR(.data_rel_ro);
    USER_READ_ONLY_END = ADDR(.data_rel_ro) + SIZEOF(.data_rel_ro);

    .data  : ALIGN(0x1000) {
        *(.data)
        *(.data.*)
    }

    .got : ALIGN(0x1000) {
        *(.got)
        *(.got.*)
    }

    .rodata : ALIGN(0x1000) {
        *(.rodata)
        *(.rodata.*)
    }

    USER_READABLE_START = ADDR(.got);
    USER_READABLE_END = ADDR(.rodata) + SIZEOF(.rodata);

    .bss : ALIGN(0x1000) {
        *(.bss)
        *(.bss.*)
    }
}
