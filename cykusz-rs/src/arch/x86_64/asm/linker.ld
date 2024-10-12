ENTRY(start)
OUTPUT_FORMAT(elf64-x86-64)

KERNEL_OFFSET = 0xFFFFFF0000000000;

SECTIONS {

        . = 1M;

        __kernel_boot_start = .;
        .boot : AT(ADDR(.boot))
        {
                KEEP(*(.multiboot_header))
                build/arch/x86_64/asm/boot.o(.text* .rodata* .data* .bss*)
                build/arch/x86_64/asm/paging.o(.text* .rodata* .data* .bss*)
                build/arch/x86_64/asm/test.o(.text* .rodata* .data* .bss*)
                build/arch/x86_64/asm/long_mode_init.o(.text* .rodata* .data* .bss*)
        }

        . += KERNEL_OFFSET;

        . = ALIGN(4K);
        __kernel_text_start = .;

        .text : AT(ADDR(.text) - KERNEL_OFFSET)
        {
                *(.text .text.*)
        }

        . = ALIGN(4K);
        __kernel_apinit_start = .;

        .apinit : AT(ADDR(.apinit) - KERNEL_OFFSET)
        {
                KEEP(*(.apinit_trampoline))
                KEEP(*(.apinit))
        }

        . = ALIGN(4K);
        __kernel_rodata_start = .;

        .rodata : AT(ADDR(.rodata) - KERNEL_OFFSET)
        {
                *(.rodata.*)
        }

        . = ALIGN(4K);
        __kernel_tdata_start = .;

        .tdata : AT(ADDR(.tdata) - KERNEL_OFFSET)
        {
                __tdata_start = .;
                KEEP(*(.tdata*))
                KEEP(*(.tbss*))
                __tdata_end = ALIGN(8);
        }

        . = ALIGN(4K);
        __kernel_gccexc_start = .;
        .gcc_except_table : AT(ADDR(.gcc_except_table) - KERNEL_OFFSET)
        {
                *(.gcc_except_table .gcc_except_table.*)
        }

        . = ALIGN(4K);
        __kernel_data_start = .;

        .data : AT(ADDR(.data) - KERNEL_OFFSET)
        {
                build/arch/x86_64/asm/higher_half.o(.stack)
                *(.data .data.*)
        }

        . = ALIGN(4K);
        __kernel_devinit_start = .;

        .devinit : AT(ADDR(.devinit) - KERNEL_OFFSET)
        {
                KEEP(*(.devinit.plat))
                KEEP(*(.devinit.plat_2))
                KEEP(*(.devinit.drv))
        }

        __kernel_devinit_end = .;
        __kernel_devfini_start = .;

        .devfini : AT(ADDR(.devfini) - KERNEL_OFFSET)
        {
                KEEP(*(.devfini.drv))
                KEEP(*(.devfini.plat_2))
                KEEP(*(.devfini.plat))
        }

        __kernel_devfini_end = .;

        . = ALIGN(4K);
        __kernel_bss_start = .;

        .bss : AT(ADDR(.bss) - KERNEL_OFFSET)
        {
                *(.bss .bss.*)
        }
}
