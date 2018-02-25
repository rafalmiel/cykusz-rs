extern rust_main
extern __p4_table
extern gdt64.pointer
extern gdt64.data
extern gdt64

global higher_half_start

section .text
higher_half_start:
    ; Setup higher half stack
    mov rsp, stack_top

    ; Unmap lower half
    mov rax, 0
    mov [__p4_table], rax

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
    call rust_main

.loop:
    hlt
    jmp $

section .stack
stack_bottom:
    times 4096*4 db 0
stack_top:
