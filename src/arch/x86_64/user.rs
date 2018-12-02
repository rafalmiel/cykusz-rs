use kernel::mm::*;

use drivers::multiboot2;

pub const USER_CODE: VirtAddr = VirtAddr(0x40000);
pub const USER_STACK: VirtAddr = VirtAddr(0x60000);

pub fn init(mboot_info: &multiboot2::Info) {
    if let Some(mtag) = mboot_info.modules_tags().next() {
        // Allocate virt space for user task
        map_flags(USER_CODE, virt::PageFlags::USER | virt::PageFlags::WRITABLE);

        // User stack
        map_flags(USER_STACK, virt::PageFlags::USER | virt::PageFlags::WRITABLE);

        for (i, ptr) in (mtag.mod_start..mtag.mod_end).enumerate() {
            unsafe {
                (USER_CODE + i).store(
                    PhysAddr(ptr as usize).to_mapped().read::<u8>()
                );
            }
        }
    }

}

pub fn find_user_program(mboot_info: &multiboot2::Info) -> Option<(MappedAddr, usize)> {
    if let Some(mtag) = mboot_info.modules_tags().next() {
        Some((PhysAddr(mtag.mod_start as usize).to_mapped(), (mtag.mod_end - mtag.mod_start) as usize))
    } else {
        None
    }
}
