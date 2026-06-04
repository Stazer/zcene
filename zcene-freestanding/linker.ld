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

    .bss : ALIGN(0x1000) {
        *(.bss)
        *(.bss.*)
    }
}
