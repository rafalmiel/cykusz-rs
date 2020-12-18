pub mod acpi;
pub mod block;
pub mod elf;
pub mod input;
pub mod multiboot2;
pub mod net;
pub mod pci;
pub mod ps2;
pub mod video;

pub fn post_module_init() {
    pci::enumerate_pci();
}
