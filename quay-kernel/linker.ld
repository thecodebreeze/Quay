ENTRY(_start)

SECTIONS {
    . = 0xffffffff80000000;

    .text : {
        *(.text .text.*)
    }

    /* Force the location counter to the next 4K page boundary */
    . = ALIGN(4K);

    .rodata : {
        /* EXPLICITLY keep our Limine requests! */
        KEEP(*(.requests))
        *(.rodata .rodata.*)
    }

    /* Force the location counter to the next 4K page boundary */
    . = ALIGN(4K);

    .data : {
        *(.data .data.*)
    }

    /* .bss and .data share the same Read/Write permissions,
       so they don't need a 4K alignment between them. */
    .bss : {
        *(COMMON)
        *(.bss .bss.*)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}