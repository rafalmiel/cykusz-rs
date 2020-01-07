BITS 64
loop:
mov rax, 0xDEADBEEF
mov rbx, 0x10000000
loop2:
mov rcx, 0x60000
mov [rcx], rax
dec rbx
cmp rbx, 0
jne loop2
int 80
jmp loop
