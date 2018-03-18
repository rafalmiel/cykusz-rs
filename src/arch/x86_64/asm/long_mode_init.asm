global long_mode_start

extern setup_SSE
extern higher_half_start
extern higher_half_start_ap

section .text
bits 64
long_mode_start:
    call setup_SSE

    ; Jump to higher half
    mov rax, higher_half_start
    jmp rax

