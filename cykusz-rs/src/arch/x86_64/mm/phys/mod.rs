use ::alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::Ordering;

use spin::Once;

use crate::drivers::multiboot2;
use crate::kernel::fs::cache::{ArcWrap, WeakWrap};
use crate::kernel::fs::pcache::{PageCacheItemArc, PageCacheItemWeak};
use crate::kernel::mm::{PhysAddr, PAGE_SIZE};
use crate::kernel::sync::{LockApi, Spin, SpinGuard};

pub use self::alloc::allocate;
pub use self::alloc::allocate_order;
pub use self::alloc::deallocate;
pub use self::alloc::deallocate_order;
pub use self::alloc::free_mem;
pub use self::alloc::order_for_size;
pub use self::alloc::used_mem;

mod alloc;
mod buddy;
mod bump;
mod iter;

struct PhysPageData {
    p_cache: PageCacheItemWeak,
    vm_use_count: u32,
}

#[repr(C)]
pub struct PhysPage {
    pt_lock: Spin<()>,
    data: UnsafeCell<PhysPageData>,
}

unsafe impl Sync for PhysPage {}

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

    pub fn lock_pt(&self) -> SpinGuard<'_, ()> {
        self.pt_lock.lock()
    }

    pub fn unlink_page_cache(&self) {
        let _lock = self.lock_pt();

        unsafe {
            let this = self.this();
            this.p_cache = WeakWrap::empty();
        }
    }

    unsafe fn this(&self) -> &mut PhysPageData {
        self.data.get().as_mut().unwrap()
    }

    pub fn link_page_cache(&self, page: &PageCacheItemArc) {
        let _lock = self.lock_pt();

        unsafe {
            let this = self.this();
            this.p_cache = ArcWrap::downgrade(&page);
        }
    }

    pub fn page_item(&self) -> Option<PageCacheItemArc> {
        let _lock = self.lock_pt();

        unsafe { self.this().p_cache.upgrade() }
    }

    pub fn inc_vm_use_count(&self) {
        let _lock = self.lock_pt();

        unsafe {
            self.this().vm_use_count += 1;
        }
    }

    pub fn dec_vm_use_count(&self) -> usize {
        let _lock = self.lock_pt();

        unsafe {
            let this = self.this();

            if this.vm_use_count > 0 {
                this.vm_use_count -= 1;
            }

            this.vm_use_count as usize
        }
    }

    pub fn vm_use_count(&self) -> usize {
        let _lock = self.lock_pt();

        unsafe { self.this().vm_use_count as usize }
    }
}

impl Default for PhysPage {
    fn default() -> Self {
        PhysPage {
            pt_lock: Spin::new(()),
            data: UnsafeCell::new(PhysPageData {
                p_cache: PageCacheItemWeak::empty(),
                vm_use_count: 0,
            }),
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
