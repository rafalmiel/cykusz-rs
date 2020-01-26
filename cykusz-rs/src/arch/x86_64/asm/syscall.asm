global asm_syscall_handler
global asm_sysretq
global asm_sysretq_userinit

extern fast_syscall_handler

asm_syscall_handler:
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    swapgs

    mov [gs:28], rsp  ; Temporarily save user stack
    mov rsp, [gs:4]   ; Set kernel stack

    mov r12, qword [gs:28]
    mov qword [gs:28], 0

    swapgs

    sti

    push r12                ; Push user stack
    push rcx                ; Push return value
    push r11                ; Push rflags

    cld

    push rdx
    push rsi
    push rdi
    push rax

    mov rdi, rsp

    call fast_syscall_handler

    add rsp, 8              ; Preserve syscall return value in rax
    pop rdi
    pop rsi
    pop rdx

asm_sysretq:
    cli

    pop r11                 ; Restore rflags
    pop rcx                 ; Restore return value

    swapgs
    mov rdx, rsp
    add rdx, 8
    mov [gs:4], rdx
    swapgs

    pop rsp                 ; Restore user stack

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    o64 sysret

asm_sysretq_userinit:
    pop r11                 ; Restore rflags
    pop rcx                 ; Restore return value
    pop rsp                 ; Restore user stack

    o64 sysret
