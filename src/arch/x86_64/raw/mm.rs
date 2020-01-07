use core::ops::*;

use crate::arch::x86_64::mm::PAGE_SIZE;
use crate::arch::x86_64::mm::phys::PhysPage;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct VirtAddr(pub usize);
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct PhysAddr(pub usize);
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct MappedAddr(pub usize);

enable_unsigned_ops!(VirtAddr);
enable_unsigned_ops!(PhysAddr);
enable_unsigned_ops!(MappedAddr);

impl PhysAddr {
    pub fn to_phys_page(&self) -> Option<&'static PhysPage> {
        if crate::arch::mm::phys::pages().is_none() {
            None
        } else {
            let idx = self.align_down(PAGE_SIZE).0 / PAGE_SIZE;

            return Some(&crate::arch::mm::phys::pages().unwrap()[idx]);
        }
    }
}

pub unsafe fn flush(addr: usize) {
    asm!("invlpg ($0)" :: "r" (addr) : "memory");
}

/// Invalidate the TLB completely by reloading the CR3 register.
///
/// # Safety
/// This function is unsafe as it causes a general protection fault (GP) if the current privilege
/// level is not 0.
pub unsafe fn flush_all() {
    use crate::arch::raw::ctrlregs::{cr3, cr3_write};
    cr3_write(cr3())
}

pub fn enable_nxe_bit() {
    use crate::arch::raw::msr::{rdmsr, wrmsr, IA32_EFER};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

pub fn enable_write_protect_bit() {
    use crate::arch::raw::ctrlregs::{cr0, cr0_write, Cr0};

    unsafe { cr0_write(cr0() | Cr0::CR0_WRITE_PROTECT) };
}
