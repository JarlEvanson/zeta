/* Tell the linker that we want an x86_64 ELF64 output file */
OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)

/* We want the symbol _start to be our entry point */
ENTRY(_start)

/* Define the program headers we want so the bootloader gives us the right */
/* MMU permissions */
PHDRS
{
    text        PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    rodata      PT_LOAD    FLAGS((1 << 2)) ;            /* Read only */
    data        PT_LOAD    FLAGS((1 << 1) | (1 << 2)) ; /* Write + Read */
    boot_text   PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    boot_rodata PT_LOAD    FLAGS((1 << 2)) ;            /* Read only */
    boot_data   PT_LOAD    FLAGS((1 << 1) | (1 << 2)) ; /* Write + Read */
    dynamic     PT_DYNAMIC FLAGS((1 << 1) | (1 << 2)) ; /* Dynamic PHDR for relocations */
}

SECTIONS
{
    /* We wanna be placed in the topmost 2GiB of the address space because
    /* that is what the Limine spec mandates. */
    /* Any address in this region will do, but often 0xffffffff80000000 is chosen as */
    /* that is the beginning of the region. */
    . = 0xffffffff80000000;

    .text ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.text .text.*)
    } :text

    .rodata ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.rodata .rodata.*)
    } :rodata

    .data ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    .dynamic : {
        *(.dynamic)
    } :dynamic

    .boot.text ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        KEEP(*(.limine.entry));
        *(.boot.text .boot.text.*)
    } :boot_text

    .boot.rodata ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        *(.boot.rodata .boot.rodata.*)
    } :boot_rodata

    .boot.data ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        KEEP(*(.limine.requests .limine.requests.*))
        *(.boot.data .boot.data.*)
    } :boot_data

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}