BITS 64
loop:
mov rax, 0xDEADBEEF
int 80
jmp loop