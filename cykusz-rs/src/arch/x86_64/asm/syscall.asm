global asm_syscall_handler
global asm_sysretq
global asm_sysretq_userinit
global asm_sysretq_forkinit
global asm_jmp_user

extern fast_syscall_handler
extern restore_user_fs

global asm_update_kern_fs_base

%include "cykusz-rs/src/arch/x86_64/asm/regs.inc"

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
    swapgs

    mov [gs:28], rsp  ; Temporarily save user stack
    mov rsp, [gs:4]   ; Set kernel stack

    sub rsp, 8
    push rax
    mov rax, qword [gs:28]
    mov qword [gs:28], 0
    mov [rsp + 8], rax
    pop rax

    call update_kern_fs_base_locked

    swapgs

    ; push rax - user stack pointer pushed earlier
    push rcx ; syscall frame
    push r11

    pushAll

    mov rdi, rsp            ; Param: pointer to syscall frame
    add rdi, 128
    mov rsi, rsp            ; Param: pointer to regs

    cld
    call fast_syscall_handler

    cli
    call restore_user_fs

    popAll

asm_sysretq:

    pop r11     ; Restore rflags
    pop rcx     ; Restore rip

    push rdx
    swapgs
    mov rdx, rsp
    add rdx, 16         ; Skip rdx and user rsp currently on the stack
    mov [gs:4], rdx     ; Stash kernel stack
    swapgs
    pop rdx

    pop rsp     ; Restore user stack

    o64 sysret

asm_sysretq_forkinit:
    cli
    call restore_user_fs

    popAll

    jmp asm_sysretq

asm_sysretq_userinit:
    cli
    call restore_user_fs

    jmp asm_sysretq

asm_jmp_user:
    push rdi    ; Param: user stack
    push rsi    ; Param: entry
    push rdx    ; Param: rflags

    cli
    call restore_user_fs

    pop r11
    pop rcx
    pop rsp

    o64 sysret
