global interrupt_handlers

extern update_kern_fs_base

section .text
bits 64

%include "cykusz-rs/src/arch/x86_64/asm/regs.inc"

%macro ISR_NOERRCODE 1
	[global isr%1]
	isr%1:
	    push 0
	    push %1

		jmp isr_common
%endmacro

%macro ISR_ERRCODE 1
	[global isr%1]
	isr%1:
	    push %1

		jmp isr_common
%endmacro

ISR_NOERRCODE 0
ISR_NOERRCODE 1
ISR_NOERRCODE 2
ISR_NOERRCODE 3
ISR_NOERRCODE 4
ISR_NOERRCODE 5
ISR_NOERRCODE 6
ISR_NOERRCODE 7
ISR_ERRCODE 8
ISR_ERRCODE 10
ISR_ERRCODE 11
ISR_ERRCODE 12
ISR_ERRCODE 13
ISR_ERRCODE 14
ISR_NOERRCODE 16
ISR_ERRCODE 17
ISR_NOERRCODE 18
ISR_NOERRCODE 19
ISR_NOERRCODE 20
ISR_ERRCODE 30

%assign i 32
%rep    224
ISR_NOERRCODE i
%assign i i+1
%endrep

extern isr_handler

; isr_handler(int_num, err_code, irq_frame, regs)
isr_common:
    pushAll

    call update_kern_fs_base

    ; prepare parameters
    mov rdi, qword [rsp + 120] ; int num value
    mov rsi, qword [rsp + 128] ; err code value
    mov rdx, rsp              ; int frame ptr
    add rdx, 136
    mov rcx, rsp              ; regs frame ptr

    sti

    cld
    call isr_handler

    popAll
    add rsp, 16             ; Remove err code & interrupt ID.

    iretq

section .rodata
interrupt_handlers:
    dq isr0
    dq isr1
    dq isr2
    dq isr3
    dq isr4
    dq isr5
    dq isr6
    dq isr7
    dq isr8
    dq 0
    dq isr10
    dq isr11
    dq isr12
    dq isr13
    dq isr14
    dq 0
    dq isr16
    dq isr17
    dq isr18
    dq isr19
    dq isr20
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq isr30                    ; int_entry_30
    dq 0
%assign i 32
%rep    224
    dq isr%+i
%assign i i+1
%endrep