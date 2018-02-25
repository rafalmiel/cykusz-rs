global setup_page_tables
global enable_paging
global __p4_table

section .text
bits 32

; kernel phys mem start 0xffff800000000000
; kernel code mem start 0xffffff0000000000
; kernel heap mem start 0xfffff80000000000


; esp - 3: offset (32bit high half)
; esp - 2: offset (32bit low half)
; esp - 1: page address
map_2mb_pages:
    push eax
    push ebx
    push ecx
    push edx

    mov ebx, [esp + 4*5]
    mov ecx, [esp + 4*6]

    mov edx, 0
.map:
    ; 2MB page size (to calculate offset)
    mov eax, 0x200000
    push edx
    ; Calculate current offset
    mul edx
    pop edx

    ; Add offset from param
    add eax, ecx
    ; Hugepage + writable + present
    or eax, 0b10000011

    ; Write page table entry
    mov [ebx + edx * 8], eax

    push eax
    mov eax, [esp + 4*8]
    ; Write high half offset value from param
    mov [ebx + edx * 8 + 4], eax
    pop eax

    inc edx

    ; Did we map all 512 entries?
    cmp edx, 512
    jne .map

    pop edx
    pop ecx
    pop ebx
    pop eax

    ret

setup_page_tables:
    ; map first P4 entry to P3 table
    mov eax, __p3_table
    or eax, 0b011		; present + writable
    mov [__p4_table], eax

    ; Entry for higher half kernel mapping at 0xffffff0000000000
    mov eax, __p3_table_high
    or eax, 0b11        ; present + writable
    mov [__p4_table + 510 * 8], eax

    ; Entry for physical mem kernel mapping at 0xffff800000000000
    mov eax, __p3_table_phys
    or eax, 0b011       ; present + writable
    mov [__p4_table + 256 * 8], eax

    mov eax, __p2_table
    or eax, 0b11
    mov [__p3_table], eax

    mov eax, __p2_table_high
    or eax, 0b11		; present + present + writable
    mov [__p3_table_high], eax

    ; Map boot mapping to 2MB huge pages
    push 0
    push 0
    push __p2_table
    call map_2mb_pages
    add esp, 4*3 ; Stack cleanup

    ; Map kernel code higher half to 2MB huge pages
    push 0
    push 0
    push __p2_table_high
    call map_2mb_pages
    add esp, 4*3 ; Stack cleanup

    ; Map RAM identity mapping in kernel higher half
    mov ecx, 0
map_p3_table_phys:
    ; Calculate offset
    mov eax, 0x40000000
    mul ecx

    ; Push overflow for map_2mb_pages call
    push edx
    ; Push offset for map_2mb_pages call
    push eax

    ; Calculate offset in __p2_table_phys
    mov ebx, __p2_table_phys
    mov eax, ecx
    mov edx, 4096
    mul edx
    add ebx, eax
    ; Push address of table for map_2mb_pages call
    push ebx
    
    ; Mark as writable and present
    or ebx, 0b11
    mov [__p3_table_phys + 8*ecx], ebx

    ; Map RAM identity mapping in kernel higher half with 2MB pages
    call map_2mb_pages
    add esp, 4*3

    inc ecx

    ; Did we map all 16 pages?
    cmp ecx, 16
    jne map_p3_table_phys
    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, __p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    or eax, 1 << 16
    mov cr0, eax

    ret

section .bss
align 4096
; Root kernel top page table level
__p4_table:
    resb 4096
; Boot mem identity mapping
__p3_table:
    resb 4096
; Kernel physical RAM identity mapping
__p3_table_phys:
    resb 4096
; Kernel higher half mapping
__p3_table_high:
    resb 4096
; Boot mem identity mapping (1GB)
__p2_table:
    resb 4096
; Kernel physical RAM identity mapping (16*512*2MB = 16GB)
__p2_table_phys:
    resb 16*4096
; Kernel higher half mapping (1GB)
__p2_table_high:
    resb 4096
