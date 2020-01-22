global start
global gdt64_code_offset
global error
global trampoline
global apinit_start
global apinit_end

extern long_mode_start
extern test_multiboot
extern test_cpuid
extern test_long_mode
extern setup_page_tables
extern enable_paging
extern __p4_table
extern setup_SSE
extern higher_half_start_ap

section .apinit_trampoline
trampoline:
    .ready: dq 0
    .cpu_num: db 0
    .stack_ptr: dq 0
    .page_table: dq 0

tmp_gdt:
    ; null descriptor 0x00
    dq 0

.code:
    ; 64-bit kernel code descriptor 0x08
    dw 0xFFFF
    dw 0
    db 0
    db 10011010b
    db 10101111b
    db 0

.pointer:
    dw $ - tmp_gdt - 1
    dq 0xE00 + (tmp_gdt - trampoline)
tmp_gdt_end:
    times 512 - ($ - tmp_gdt_end) db 0

section .apinit
bits 16
apinit_start:
    cli

    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Ap boot stack - temporary - used by one cpu at a time
    mov sp, 0x4000

    lgdt [0xE00 + tmp_gdt.pointer - trampoline]

    ; Enable:
    ; bit 4 - Page Size Extension
    ; bit 5 - Physical Address Extension
    mov eax, 0x30
    mov cr4, eax

    ; go to long mode
    mov eax, __p4_table
    mov cr3, eax

    ; Enable long mode in EFER register
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr

    ; enable paging in the cr0 register
    ; bit 0 - Protected Mode Enable
    ; bit 16 - Write Protect
    ; bit 31 - Paging
    mov eax, cr0
    or eax, 1 << 31
    or eax, 1 << 16
    or eax, 1
    mov cr0, eax

    jmp 0x08:0x1000 + (long_mode_start_ap - apinit_start)

long_mode_start_ap:
bits 64
    mov rax, higher_half_start_ap
    jmp rax

.apinit_hlt:
    hlt
    jmp .apinit_hlt
apinit_end:

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