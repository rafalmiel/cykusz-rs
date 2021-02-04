use ::alloc::vec::Vec;
use core::sync::atomic::Ordering;

use spin::Once;

use crate::drivers::multiboot2;
use crate::kernel::fs::cache::{ArcWrap, WeakWrap};
use crate::kernel::fs::pcache::{PageItem, PageItemWeak};
use crate::kernel::mm::{PhysAddr, PAGE_SIZE};
use crate::kernel::sync::{Spin, SpinGuard};

pub use self::alloc::allocate;
pub use self::alloc::allocate_order;
pub use self::alloc::deallocate;
pub use self::alloc::deallocate_order;
pub use self::alloc::free_mem;
pub use self::alloc::used_mem;

mod alloc;
mod buddy;
mod bump;
mod iter;

#[repr(C)]
pub struct PhysPage {
    pt_lock: Spin<()>,
    p_cache: PageItemWeak,
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

    pub fn lock_pt(&self) -> SpinGuard<()> {
        self.pt_lock.lock()
    }

    pub fn unlink_page_cache(&self) {
        unsafe {
            let _lock = self.pt_lock.lock();

            let this = &mut *(self as *const _ as *mut PhysPage);
            this.p_cache = WeakWrap::empty();
        }
    }

    pub fn link_page_cache(&self, page: &PageItem) {
        unsafe {
            let _lock = self.pt_lock.lock();

            let this = &mut *(self as *const _ as *mut PhysPage);
            this.p_cache = ArcWrap::downgrade(&page);
        }
    }

    pub fn page_item(&self) -> Option<PageItem> {
        let _lock = self.pt_lock.lock();

        self.p_cache.upgrade()
    }
}

impl Default for PhysPage {
    fn default() -> Self {
        PhysPage {
            pt_lock: Spin::new(()),
            p_cache: PageItemWeak::empty(),
        }
    }
}

pub static PAGES: Once<Vec<PhysPage>> = Once::new();

pub fn pages() -> Option<&'static Vec<PhysPage>> {
    PAGES.get()
}

pub fn init_pages() {
    PAGES.call_once(|| {
        let mut v = Vec::<PhysPage>::new();
        v.resize_with(
            alloc::NUM_PAGES.load(Ordering::SeqCst) as usize,
            Default::default,
        );

        println!(
            "PhysPage size: {} num {}",
            core::mem::size_of::<PhysPage>(),
            v.len()
        );
        v
    });
}

pub fn init(mboot_info: &multiboot2::Info) {
    alloc::init(mboot_info);
}
