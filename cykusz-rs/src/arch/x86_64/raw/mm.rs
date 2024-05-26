use core::arch::asm;
use core::ops::*;

use crate::arch::mm::virt::p4_table_addr;
use crate::arch::mm::virt::table::P4Table;
use crate::arch::x86_64::mm::PAGE_SIZE;
use crate::arch::x86_64::mm::phys::PhysPage;
use crate::kernel::mm::virt::PageFlags;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default, Hash)]
#[repr(transparent)]
pub struct VirtAddr(pub usize);

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default, Hash)]
#[repr(transparent)]
pub struct PhysAddr(pub usize);

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Default, Hash)]
#[repr(transparent)]
pub struct MappedAddr(pub usize);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct UserAddr {
    page_table: PhysAddr,
    addr: VirtAddr,
}

enable_unsigned_ops!(VirtAddr);
enable_unsigned_ops!(PhysAddr);
enable_unsigned_ops!(MappedAddr);

impl PhysAddr {
    pub fn to_phys_page(&self) -> Option<&'static PhysPage> {
        if let Some(pages) = crate::arch::mm::phys::pages() {
            let idx = self.align_down(PAGE_SIZE).0 / PAGE_SIZE;

            if idx >= pages.len() {
                return None;
            }

            return Some(&pages[idx]);
        }

        None
    }
}

impl From<VirtAddr> for UserAddr {
    fn from(addr: VirtAddr) -> Self {
        Self {
            page_table: p4_table_addr(),
            addr,
        }
    }
}

impl From<UserAddr> for VirtAddr {
    fn from(addr: UserAddr) -> Self {
        addr.addr
    }
}

impl UserAddr {
    pub fn update_flags(&self, flags: PageFlags) -> Option<PhysAddr> {
        let ptable = P4Table::new_mut_at_phys(self.page_table);

        let res = ptable.update_flags(self.addr, flags);

        if p4_table_addr() == self.page_table {
            unsafe { flush(self.addr.0) }
        }

        res
    }

    pub fn insert_flags(&self, flags: PageFlags) -> Option<PhysAddr> {
        let ptable = P4Table::new_mut_at_phys(self.page_table);

        let res = ptable.insert_flags(self.addr, flags);

        if p4_table_addr() == self.page_table {
            unsafe { flush(self.addr.0) }
        }

        res
    }

    pub fn remove_flags(&self, flags: PageFlags) -> Option<PhysAddr> {
        let ptable = P4Table::new_mut_at_phys(self.page_table);

        let res = ptable.remove_flags(self.addr, flags);

        if p4_table_addr() == self.page_table {
            unsafe { flush(self.addr.0) }
        }

        res
    }
}

pub unsafe fn flush(addr: usize) {
    asm!("invlpg [{0}]", in(reg) addr);
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
    use crate::arch::raw::msr::{IA32_EFER, rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

pub fn enable_write_protect_bit() {
    use crate::arch::raw::ctrlregs::{cr0, Cr0, cr0_write};

    unsafe { cr0_write(cr0() | Cr0::CR0_WRITE_PROTECT) };
}
