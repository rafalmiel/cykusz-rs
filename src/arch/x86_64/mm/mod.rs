pub mod phys;
pub mod virt;
pub mod heap;

pub use arch::raw::mm::{MappedAddr, PhysAddr, VirtAddr};

use drivers::multiboot2;

const VIRT: VirtAddr = VirtAddr(0xFFFFFF0000000000);
const PHYSMAP: MappedAddr = MappedAddr(0xFFFF800000000000);

pub const PAGE_SIZE: usize = 4096;

impl PhysAddr {
    pub fn to_mapped(&self) -> MappedAddr {
        if self.0 < PHYSMAP.0 {
            MappedAddr(self.0 + PHYSMAP.0)
        } else {
            MappedAddr(self.0)
        }
    }

    pub fn to_virt(&self) -> VirtAddr {
        if self.0 < PHYSMAP.0 {
            VirtAddr(self.0 + VIRT.0)
        } else {
            VirtAddr(self.0)
        }
    }
}

impl VirtAddr {
    pub fn to_phys(&self) -> PhysAddr {
        if self >= &VIRT {
            PhysAddr(self.0 - VIRT.0)
        } else {
            PhysAddr(self.0)
        }
    }
}

impl MappedAddr {
    pub fn to_phys(&self) -> PhysAddr {
        if self >= &PHYSMAP {
            PhysAddr(self.0 - PHYSMAP.0)
        } else {
            PhysAddr(self.0)
        }
    }
}

pub fn init(mboot: &multiboot2::Info) {
    phys::init(&mboot);

    println!("[ OK ] Physical Memory Initialised");

    virt::init(&mboot);

    println!("[ OK ] Virtual Memory Initialised");
}