global test_cpuid
global test_long_mode
global test_multiboot

extern error

section .text
bits 32

test_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21) in
    ; the FLAGS register. if we can flip it, CPUID is available

    ; Copy FALGS into EAX via stack
    pushfd
    pop eax

    ; Copy to ECX as well for comparing later on
    mov ecx, eax

    ; Flip the ID bit
    xor eax, 1 << 21

    ; Copy EAX to FLAGS via the stack
    push eax
    popfd

    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the bit
    ; back if it was ever flipped)
    push ecx
    popfd

    ; Compare EAX and ECX. If they are eaual that means the bit wasn't
    ; flipped, and CPUID isn't supported
    xor eax, ecx
    jz .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error

test_long_mode:
    ; test if extended processor info in available

    ; Set the A-register to 0x80000000
    mov eax, 0x80000000

    ; CPU identification
    cpuid

    ; Compare the A-register with 0x80000001
    cmp eax, 0x80000001

    ; It is less, there is no long mode
    jb .no_long_mode

    ; Set the A-register to 0x80000001
    mov eax, 0x80000001

    ; CPU identification
    cpuid

    ; Test if the LM-bit, which is bit 29, is set in the D-register
    test edx, 1 << 29

    ; They aren't, there is no long mode
    jz .no_long_mode

    ; Test for the availability of apic
    mov eax, 0x1
    cpuid
    test edx, 1 << 9
    jz .no_apic
    ret
.no_long_mode:
    mov al, "2"
    jmp error
.no_1gb_pages:
    mov al, "3"
    jmp error
.no_apic:
    mov al, "4"
    jmp error

test_multiboot:
    ; Test for magic value written by the bootloader into eax before loading the kernel
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error
