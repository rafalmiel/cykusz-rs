use crate::arch::raw::ctrlregs;
use crate::arch::raw::mm;
use crate::kernel::mm::{PhysAddr, VirtAddr};
use crate::kernel::mm::PAGE_SIZE;
use crate::kernel::mm::virt;

use self::table::*;

pub mod entry;
mod page;
pub mod table;

pub fn p4_table_addr() -> PhysAddr {
    unsafe { PhysAddr(ctrlregs::cr3() as usize) }
}

pub fn current_p4_table() -> &'static mut P4Table {
    p4_table(p4_table_addr())
}

pub fn p4_table(addr: PhysAddr) -> &'static mut P4Table {
    P4Table::new_mut_at_phys(addr)
}

pub fn flush(virt: VirtAddr) {
    unsafe {
        mm::flush(virt.0);
    }
}

pub fn flush_all() {
    unsafe {
        mm::flush_all();
    }
}

pub fn map_flags(virt: VirtAddr, flags: virt::PageFlags) {
    current_p4_table().map_flags(virt, flags);

    flush(virt);
}

pub fn map_to_flags(virt: VirtAddr, phys: PhysAddr, flags: virt::PageFlags) {
    current_p4_table().map_to_flags(virt, phys, flags);

    flush(virt);
}

pub fn get_flags(virt: VirtAddr) -> Option<crate::arch::mm::virt::entry::Entry> {
    current_p4_table().get_flags(virt)
}

pub fn update_flags(virt: VirtAddr, flags: virt::PageFlags) -> bool {
    let res = current_p4_table().update_flags(virt, flags);

    flush(virt);

    return res.is_some();
}

pub fn map(virt: VirtAddr) {
    map_flags(virt, virt::PageFlags::WRITABLE);
}

pub fn unmap(virt: VirtAddr) {
    current_p4_table().unmap(virt);

    flush(virt);
}

#[allow(unused)]
pub fn map_to(virt: VirtAddr, phys: PhysAddr) {
    current_p4_table().map_to(virt, phys);

    flush(virt);
}

pub fn to_phys(addr: VirtAddr) -> Option<PhysAddr> {
    current_p4_table().to_phys(addr)
}

pub unsafe fn activate_table(table: &P4Table) {
    ctrlregs::cr3_write(table.phys_addr().0 as u64);
}

fn remap(mboot_info: &crate::drivers::multiboot2::Info) {
    let table = P4Table::new();

    for elf in mboot_info.elf_tag().unwrap().sections() {
        let s = elf.address().align_down(PAGE_SIZE);
        let e = elf.end_address().align_up(PAGE_SIZE);

        use crate::drivers::multiboot2::elf::ElfSectionFlags;

        if !elf.flags.contains(ElfSectionFlags::ALLOCATED) {
            continue;
        }

        let flags = virt::PageFlags::from(elf.flags as ElfSectionFlags);

        for addr in (s..e).step_by(PAGE_SIZE) {
            table.map_to_flags(addr, addr.to_phys(), flags);
        }
    }

    // Set physmap from previous mapping
    let orig = current_p4_table();
    table.set_entry(256, orig.entry_at(256));

    unsafe {
        activate_table(&table);
    }
}

pub fn init(mboot_info: &crate::drivers::multiboot2::Info) {
    use crate::arch::raw::mm::{enable_nxe_bit, enable_write_protect_bit};
    enable_nxe_bit();
    enable_write_protect_bit();

    println!("[ OK ] NXE and Write Protect Enabled");

    remap(&mboot_info);

    println!("[ OK ] Kernel Code Remapped");
}
