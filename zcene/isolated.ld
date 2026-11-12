ENTRY(_start)

PHDRS {
    code PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS {
    . = 0x8000000000;

    .text : {
        *(.text .text.*)
    } :code

    .rodata : {
        *(.rodata .rodata.*)
    } :code

    .got : {
        *(.got .got.*)
    } :code

    . = ALIGN(0x1000);

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame*)
        *(.comment)
        *(.note*)
    }
}
