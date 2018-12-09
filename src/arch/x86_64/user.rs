use kernel::mm::*;

use drivers::multiboot2;

use core::sync::atomic::{AtomicU64, ATOMIC_U64_INIT, Ordering};

static USER_PROGRAM: AtomicU64 = ATOMIC_U64_INIT;
static USER_PROGRAM_SIZE: AtomicU64 = ATOMIC_U64_INIT;

pub fn init(mboot_info: &multiboot2::Info) {
    if let Some(mtag) = mboot_info.modules_tags().next() {
        USER_PROGRAM.store(PhysAddr(mtag.mod_start as usize).to_mapped().0 as u64, Ordering::SeqCst);
        USER_PROGRAM_SIZE.store((mtag.mod_end - mtag.mod_start) as u64, Ordering::SeqCst);
    }
}

pub fn get_user_program() -> MappedAddr {
    MappedAddr(USER_PROGRAM.load(Ordering::SeqCst) as usize)
}

pub fn get_user_program_size() -> u64 {
    USER_PROGRAM_SIZE.load(Ordering::SeqCst)
}
