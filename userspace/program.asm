BITS 64
loop:
mov rax, 0xDEADBEEF
mov rbx, 0x1000000
loop2:
dec rbx
cmp rbx, 0
jne loop2
int 80
jmp loop
