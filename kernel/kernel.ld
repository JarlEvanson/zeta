/* Produce a x86_64 ELF64 output file */
OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)

/* _start will be our entry point */
ENTRY(_start)

PHDRS {
    text    PT_LOAD     FLAGS((1 << 0) | (1 << 2));     /* Execute + Read */ 
    rodata  PT_LOAD     FLAGS((1 << 2));                /* Read only */
    data    PT_LOAD     FLAGS((1 << 1) | (1 << 2));     /* Write + Read */
    dynamic PT_DYNAMIC  FLAGS((1 << 1) | (1 << 2));     /* Write + Read */
}

SECTIONS {
    . = 0xFFFFFFFF80000000;

    /* Align to next page */
    .text ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.text .text.*)
    } :text

    /* Align to next page */
    .rodata ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.rodata .rodata.*)
    } :rodata

    /* Align to next page */
    .data ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.data .data.*)
    } :data

    .dynamic : {
        *(.dynamic .dynamic.*)
    } :data :dynamic

    .bss : {
        *(.bss .bss.*)
    } :data

    /DISCARD/ : {
        *(.eh_frame .eh_frame.*)
        *(.note .note.*)
    }
}