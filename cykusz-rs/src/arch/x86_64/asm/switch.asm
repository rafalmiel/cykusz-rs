global switch_to
global activate_to
global isr_return

section .text
bits 64
; fn switch_to(old: *mut *mut Context, new: *mut Context)
; old = rdi
; new = rsi
switch_to:
    push r15
    push r14
    push r13
    push r12
    push rbx
    push rbp
    pushfq			; push regs to current ctx

    mov rax, cr3    ; Save CR3
    push rax

    mov [rdi], rsp	; update old ctx ptr with current stack ptr
    mov rsp, rsi	; switch to new stack

    pop rax         ; Restore CR3
    mov cr3, rax

    popfq
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15

    ret

activate_to:
    mov rsp, rsi	; switch to new stack

    pop rax         ; Restore CR3
    mov cr3, rax

    popfq
    pop rbp
    pop rbx
    pop r12
    pop r13
    pop r14
    pop r15

    ret

isr_return:
    pop rdi         ; Param passed to the function
    iretq
