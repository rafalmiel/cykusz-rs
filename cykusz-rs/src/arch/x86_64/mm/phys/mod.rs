use ::alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Once;

use crate::drivers::multiboot2;
use crate::kernel::mm::{PhysAddr, PAGE_SIZE};
use crate::kernel::sync::{Mutex, MutexGuard};

pub use self::alloc::allocate;
pub use self::alloc::deallocate;

mod alloc;
mod iter;

#[repr(C)]
pub struct PhysPage {
    pt_lock: Mutex<()>,
}

impl PhysPage {
    fn base_addr() -> PhysAddr {
        PhysAddr(&pages().unwrap()[0] as *const _ as usize)
    }

    fn this_addr(&self) -> PhysAddr {
        PhysAddr(self as *const _ as usize)
    }

    pub fn to_phys_addr(&self) -> PhysAddr {
        (self.this_addr() - Self::base_addr()) / core::mem::size_of::<Self>() * PAGE_SIZE
    }

    pub fn lock_pt(&self) -> MutexGuard<()> {
        self.pt_lock.lock()
    }
}

impl Default for PhysPage {
    fn default() -> Self {
        PhysPage {
            pt_lock: Mutex::new(()),
        }
    }
}

pub static PAGES: Once<Vec<PhysPage>> = Once::new();

pub fn pages() -> Option<&'static Vec<PhysPage>> {
    PAGES.r#try()
}

pub fn init_pages() {
    PAGES.call_once(|| {
        let mut v = Vec::<PhysPage>::new();
        v.resize_with(
            alloc::NUM_PAGES.load(Ordering::SeqCst) as usize,
            Default::default,
        );
        v
    });
}

pub fn init(mboot_info: &multiboot2::Info) {
    alloc::init(mboot_info);
}