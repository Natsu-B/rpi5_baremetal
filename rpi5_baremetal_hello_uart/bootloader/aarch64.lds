OUTPUT_ARCH(aarch64)
ENTRY(_start)

SECTIONS
{
    . = 0x200000;
    .text.boot : ALIGN(4) {
        *(.text.boot)
    }
    .text : ALIGN(8) {
        *(.text*)
        . = ALIGN(8);
    }

    .rodata : ALIGN(8) {
        *(.rodata*)
        . = ALIGN(8);
    }

    .data : ALIGN(8) {
        *(.data*)
        . = ALIGN(8);
    }

    .bss : ALIGN(8) {
        _BSS_START = .;
        *(.bss*)
        . = ALIGN(8);
        _BSS_END = .;
    }

    . = 0x4000000;
    _STACK_TOP = .;
}