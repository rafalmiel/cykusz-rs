pub mod entry;
mod page;
pub mod table;

use arch::raw::mm;
use arch::raw::ctrlregs;
use kernel::mm::virt;
use kernel::mm::allocate;
use kernel::mm::{PhysAddr,VirtAddr};
use kernel::mm::PAGE_SIZE;
use self::table::*;

pub fn p4_table_addr() -> PhysAddr {
    unsafe {
        PhysAddr(ctrlregs::cr3() as usize)
    }
}
pub fn current_p4_table() -> &'static mut P4Table {
    P4Table::new_mut_at_phys(p4_table_addr())
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

pub unsafe fn activate_table(table: &P4Table) {
    ctrlregs::cr3_write(table.phys_addr().0 as u64);
}

fn remap(mboot_info: &::drivers::multiboot2::Info) {
    let frame = allocate().expect("Out of mem!");
    let table = P4Table::new_mut(&frame);

    table.clear();

    for elf in mboot_info.elf_tag().unwrap().sections() {

        let s = elf.address().align_down(PAGE_SIZE);
        let e = elf.end_address().align_up(PAGE_SIZE);

        let mut flags = virt::PageFlags::empty();

        use ::drivers::multiboot2::elf::ElfSectionFlags;

        if (elf.flags as usize & ElfSectionFlags::Allocated as usize) == 0 as usize {
            continue;
        }

        if (elf.flags as usize & ElfSectionFlags::Writable as usize) == ElfSectionFlags::Writable as usize {
            flags.insert(virt::PageFlags::WRITABLE);
        }
        if (elf.flags as usize & ElfSectionFlags::Executable as usize) == 0 {
            flags.insert(virt::PageFlags::NO_EXECUTE);
        }

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

pub fn init(mboot_info: &::drivers::multiboot2::Info) {
    use arch::raw::mm::{enable_nxe_bit,enable_write_protect_bit};
    enable_nxe_bit();
    enable_write_protect_bit();

    println!("[ OK ] NXE and Write Protect Enabled");

    remap(&mboot_info);

    println!("[ OK ] Kernel Code Remapped");
}
