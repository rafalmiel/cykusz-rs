use bit_field::BitField;

use crate::arch::mm::virt::entry::Entry;
use crate::arch::raw::msr::{rdmsr, wrmsr, IA32_PAT};
use crate::kernel::mm::virt;

#[repr(u8)]
pub enum Mode {
    UC = 0,  // Uncacheable
    WC = 1,  // Write-Combining
    WT = 4,  // Write-Through
    WP = 5,  // Write-Protect
    WB = 6,  // Write-Back
    UCM = 7, // Uncached-
}

#[repr(transparent)]
struct Pat(u64);

impl Pat {
    pub fn new(val: u64) -> Pat {
        Pat(val)
    }

    pub fn set_pat(&mut self, pat: usize, mode: Mode) {
        assert!(pat < 8);

        let offset = pat * 8;

        self.0.set_bits(offset..offset + 4, mode as u64);
    }

    pub fn val(&self) -> u64 {
        return self.0;
    }
}

pub fn init() {
    let mut pat = unsafe { Pat::new(rdmsr(IA32_PAT)) };

    pat.set_pat(0, Mode::WB);
    pat.set_pat(1, Mode::WT);
    pat.set_pat(2, Mode::UCM);
    pat.set_pat(3, Mode::UC);
    pat.set_pat(4, Mode::WC);
    pat.set_pat(5, Mode::WP);
    pat.set_pat(6, Mode::UCM);
    pat.set_pat(7, Mode::UC);

    unsafe {
        wrmsr(IA32_PAT, pat.val());
    }
}

pub fn from_kernel_flags(is_hugepage: bool, flags: virt::PageFlags) -> Entry {
    let mut entry = Entry::new_empty();

    let pat_flag = if is_hugepage {
        Entry::HP_PAT
    } else {
        Entry::HUGE_PAGE
    };

    if flags.contains(virt::PageFlags::WRITE_COMBINE) {
        entry.insert(pat_flag); // PAT flag
    } else if flags.contains(virt::PageFlags::WRITE_PROTECT) {
        entry.insert(pat_flag | Entry::WRT_THROUGH);
    } else {
        if flags.contains(virt::PageFlags::WRT_THROUGH) {
            entry.insert(Entry::WRT_THROUGH);
        }

        if flags.contains(virt::PageFlags::NO_CACHE) {
            entry.insert(Entry::NO_CACHE);
        }
    }

    entry
}
