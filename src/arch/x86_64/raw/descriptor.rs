use crate::arch::raw::gdt;
use crate::arch::raw::idt;
use core::mem;

bitflags! {
    pub struct Flags: u8 {
        const MISSING = 0;

        // Presetnt BIT
        const PRESENT = 1 << 7;

        //Privilige bits
        const PRIV_RING0 = 0b00 << 5;
        const PRIV_RING1 = 0b01 << 5;
        const PRIV_RING2 = 0b10 << 5;
        const PRIV_RING3 = 0b11 << 5;

        const SYSTEM = 0 << 4;
        const SEGMENT = 1 << 4;

        const SYS_TYPE_32_TASK_GATE = 0b0101;
        const SYS_TYPE_16_INTERRUPT_GATE = 0b0110;
        const SYS_TYPE_16_TRAP_GATE = 0b0111;
        const SYS_TYPE_32_INTERRUPT_GATE = 0b1110;
        const SYS_TYPE_32_TRAP_GATE = 0b1111;

        const SYS_RING0_INTERRUPT_GATE = Self::PRESENT.bits |
                                         Self::PRIV_RING0.bits |
                                         Self::SYSTEM.bits |
                                         Self::SYS_TYPE_32_INTERRUPT_GATE.bits;
        const SYS_RING3_INTERRUPT_GATE = Self::PRESENT.bits |
                                         Self::PRIV_RING3.bits |
                                         Self::SYSTEM.bits |
                                         Self::SYS_TYPE_32_INTERRUPT_GATE.bits;

        // Executable bit
        const SEG_C_EXECUTABLE = 1 << 3;

        // Confirming bit for code, direction bit for data
        const SEG_D_CONFORMING = 1 << 2;
        const SEG_D_GROW_DOWN = 1 << 2;

        // RD for code, WR for data
        const SEG_C_READABLE = 1 << 1;
        const SEG_D_WRITABLE = 1 << 1;

        // Accessed bit
        const SEG_ACCESSED = 1;

        const SEG_RING0_CODE = Self::PRESENT.bits |
                               Self::PRIV_RING0.bits |
                               Self::SEGMENT.bits |
                               Self::SEG_C_EXECUTABLE.bits |
                               Self::SEG_C_READABLE.bits;
        const SEG_RING0_DATA = Self::PRESENT.bits |
                               Self::PRIV_RING0.bits |
                               Self::SEGMENT.bits |
                               Self::SEG_D_WRITABLE.bits;
    }
}

impl Flags {
    pub const fn cbits(&self) -> u8 {
        self.bits
    }
}

#[repr(C, packed)]
pub struct DescriptorTablePointer<T> {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: *const T,
}

impl<T> DescriptorTablePointer<T> {
    pub fn init(&mut self, e_slice: &[T]) {
        self.limit = (e_slice.len() * mem::size_of::<T>() - 1) as u16;
        self.base = e_slice.as_ptr();
    }

    pub const fn empty() -> Self {
        DescriptorTablePointer::<T> {
            limit: 0,
            base: ::core::ptr::null(),
        }
    }
}

pub unsafe fn lidt(idt: &DescriptorTablePointer<idt::IdtEntry>) {
    asm!("lidt ($0)" :: "r"(idt) : "memory");
}

pub unsafe fn lgdt(gdt: &DescriptorTablePointer<gdt::GdtEntry>) {
    asm!("lgdt ($0)" :: "r"(gdt) : "memory");
}
