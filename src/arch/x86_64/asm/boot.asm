global start
global gdt64_code_offset
global error

extern long_mode_start
extern test_multiboot
extern test_cpuid
extern test_long_mode
extern setup_page_tables
extern enable_paging

section .text
bits 32
start:
  cli

    mov esp, boot_stack_top
    mov edi, ebx       ;Multiboot address

    ; Save multiboot address on stack
    push edi

    call test_multiboot
    call test_cpuid
    call test_long_mode

    call setup_page_tables
    call enable_paging

    lgdt [gdt64.pointer]

    ; Restore multiboot address
    pop edi

    jmp gdt64.code:long_mode_start

error:
    mov dword [0xb8000], 0x4f524f45 ; ER
    mov dword [0xb8004], 0x4f3a4f52 ; R:
    mov dword [0xb8008], 0x4f204f20 ;
    mov byte  [0xb8008], al		; err code
    hlt

section .bss
boot_stack_bottom:
    resb 512
boot_stack_top:

%include "src/arch/x86_64/asm/gdt.inc"

; lower half gdt
section .rodata
bits 64
gdt64:
    dq 0														    ; zero entry
.code: equ $ - gdt64
istruc GDTEntry
    at GDTEntry.limitl, dw 0
    at GDTEntry.basel, dw 0
    at GDTEntry.basem, db 0
    at GDTEntry.attribute, db attrib.present | attrib.code | attrib.always_set
    at GDTEntry.flags__limith, db flags.long_mode
    at GDTEntry.baseh, db 0
iend
.pointer:
    dw $ - gdt64 - 1
    dq gdt64