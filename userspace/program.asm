BITS 64
section .text
global _start
_start:
loop:
mov rax, 0xDEADBEEF
mov rbx, 0x10000000
loop2:
push rax
mov rcx, 0x600000
mov [rcx], rax
dec rbx
cmp rbx, 0
pop rax
jne loop2
int 80
jmp loop
