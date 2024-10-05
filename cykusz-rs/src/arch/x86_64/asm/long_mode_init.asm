global long_mode_start

extern setup_SSE
extern higher_half_start

section .text
bits 64
long_mode_start:
    ; Jump to higher half
    mov rax, higher_half_start
    jmp rax

