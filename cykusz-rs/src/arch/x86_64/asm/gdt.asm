global gdt64
global gdt64.code
global gdt64.data
global gdt64.pointer

%include "cykusz-rs/src/arch/x86_64/asm/gdt.inc"

; higher half gdt
section .data
bits 64
gdt64:
    dq 0    ; zero entry
.code: equ $ - gdt64
istruc GDTEntry
    at GDTEntry.limitl, dw 0
    at GDTEntry.basel, dw 0
    at GDTEntry.basem, db 0
    at GDTEntry.attribute, db attrib.always_set | attrib.present | attrib.code | attrib.readable
    at GDTEntry.flags__limith, db flags.long_mode
    at GDTEntry.baseh, db 0
iend

.data: equ $ - gdt64
istruc GDTEntry
    at GDTEntry.limitl, dw 0
    at GDTEntry.basel, dw 0
    at GDTEntry.basem, db 0
; AMD System Programming Manual states that the writeable bit is ignored in long mode, but ss can not be set to this descriptor without it
    at GDTEntry.attribute, db attrib.always_set | attrib.present | attrib.writable
    at GDTEntry.flags__limith, db 0
    at GDTEntry.baseh, db 0
iend
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
