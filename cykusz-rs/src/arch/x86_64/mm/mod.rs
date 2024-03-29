pub use crate::arch::raw::mm::{MappedAddr, PhysAddr, VirtAddr};
use crate::drivers::multiboot2;

pub mod heap;
pub mod pat;
pub mod phys;
pub mod virt;

const VIRT: VirtAddr = VirtAddr(0xFFFFFF0000000000);
const MAPPED: MappedAddr = MappedAddr(0xFFFF800000000000);
pub const MAX_USER_ADDR: VirtAddr = VirtAddr(0x8000_0000_0000);
pub const MMAP_USER_ADDR: VirtAddr = VirtAddr(0x7000_0000_0000);

pub const PAGE_SIZE: usize = 4096;

impl PhysAddr {
    pub const fn to_mapped(&self) -> MappedAddr {
        if self.0 < MAPPED.0 {
            MappedAddr(self.0 + MAPPED.0)
        } else {
            MappedAddr(self.0)
        }
    }

    pub fn to_virt(&self) -> VirtAddr {
        if self.0 < MAPPED.0 {
            VirtAddr(self.0 + VIRT.0)
        } else {
            VirtAddr(self.0)
        }
    }
}

impl VirtAddr {
    pub const fn to_phys(&self) -> PhysAddr {
        if self.0 >= VIRT.0 {
            PhysAddr(self.0 - VIRT.0)
        } else {
            PhysAddr(self.0)
        }
    }

    pub fn to_phys_pagewalk(&self) -> Option<PhysAddr> {
        crate::kernel::mm::to_phys(*self)
    }

    pub fn is_user(&self) -> bool {
        *self < MAX_USER_ADDR
    }
}

impl MappedAddr {
    pub const fn to_phys(&self) -> PhysAddr {
        if self.0 >= MAPPED.0 {
            PhysAddr(self.0 - MAPPED.0)
        } else {
            PhysAddr(self.0)
        }
    }

    pub const fn as_virt(&self) -> VirtAddr {
        VirtAddr(self.0)
    }
}

pub fn init(mboot: &multiboot2::Info) {
    phys::init(&mboot);

    println!("[ OK ] Physical Memory Initialised");

    virt::init(&mboot);

    println!("[ OK ] Virtual Memory Initialised");

    pat::init();

    println!("[ OK ] PAT Initialised");
}
