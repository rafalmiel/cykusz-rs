extern x86_64_rust_main
extern x86_64_rust_main_ap
extern __p4_table
extern gdt64.pointer
extern gdt64.data
extern gdt64
extern setup_SSE

global higher_half_start
global higher_half_start_ap

section .text
higher_half_start:
    ; Setup higher half stack
    mov rsp, stack_top

    ; switch to higher half gdt
    mov rax, gdt64.pointer
    lgdt [rax]

    ; update selectors
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; Reload page table
    mov rax, cr3
    mov cr3, rax

    ; Jump to rust code
    mov rsi, stack_top
    mov rdx, gdt64
    call x86_64_rust_main

.loop:
    hlt
    jmp $

higher_half_start_ap:
    ; switch to higher half gdt
    mov rax, gdt64.pointer
    lgdt [rax]

    ; Get stack pointer
    mov rbx, [0xE02]

    ; Get page table pointer
    mov rax, [0xE00 + 10]
    mov cr3, rax

    ; Setup higher half stack
    mov rsp, rbx

    ; update selectors
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    call x86_64_rust_main_ap

.loop:
    hlt
    jmp $

section .stack
stack_bottom:
    times 4096*16 db 0
stack_top:
