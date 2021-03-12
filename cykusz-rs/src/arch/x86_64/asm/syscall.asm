global asm_syscall_handler
global asm_sysretq
global asm_sysretq_userinit
global asm_sysretq_forkinit

extern fast_syscall_handler
extern restore_user_fs

global asm_update_kern_fs_base

update_kern_fs_base_locked:
    push rbx
    push rdx
    push rcx
    push rax

    mov rbx, qword [gs:104] ; Offset into TSS holding FS_BASE for this cpu
    mov ecx, 0xC0000100     ; IA32_FS_BASE msr
    mov eax, ebx
    shr rbx, 32
    mov edx, ebx

    wrmsr

    pop rax
    pop rcx
    pop rdx
    pop rbx

    ret


asm_update_kern_fs_base:
    pushfq

    cli

    swapgs

    call update_kern_fs_base_locked

    swapgs

    popfq

    ret

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

    call update_kern_fs_base_locked

    swapgs

    sti

    push r12                ; Push user stack
    push rcx                ; Push return value
    push r11                ; Push rflags

    push r9
    push r8
    push r10
    push rdx
    push rsi
    push rdi
    push rax

    mov rdi, rsp

    cld
    call fast_syscall_handler

    add rsp, 8              ; Preserve syscall return value in rax
    pop rdi
    pop rsi
    pop rdx
    pop r10
    pop r8
    pop r9

asm_sysretq:
    cli

    push rax                ; Preserve syscall return value
    call restore_user_fs    ; Set this tasks fs base
    pop rax

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

asm_sysretq_forkinit:
    mov rax, 0

    jmp asm_sysretq

asm_sysretq_userinit:
    call restore_user_fs    ; Switch to user fs base

    pop r11                 ; Restore rflags
    pop rcx                 ; Restore return value
    pop rsp                 ; Restore user stack

    o64 sysret


