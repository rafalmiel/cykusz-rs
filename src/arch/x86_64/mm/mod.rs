pub mod phys;

use drivers::multiboot2;

pub const PAGE_SIZE: usize = 4096;

pub fn init(mboot: &multiboot2::Info) {
    phys::init(&mboot);

    println!("[ OK ] Physical Memory Initialised");
}