use crate::drivers::multiboot2;

#[macro_use]
pub mod output;
#[macro_use]
pub mod raw;
#[macro_use]
pub mod task;
pub mod gdt;
pub mod idt;
pub mod mm;
pub mod acpi;
pub mod int;
pub mod dev;
pub mod smp;
pub mod tls;
pub mod timer;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: mm::PhysAddr) {
    output::clear();
    gdt::init();
    println!("[ OK ] GDT Initialised");

    idt::init();
    println!("[ OK ] IDT Initialised");

    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    mm::init(&mboot);

    acpi::init();

    dev::init();

    crate::rust_main();
}

#[no_mangle]
pub extern "C" fn x86_64_rust_main_ap() {
    crate::arch::raw::mm::enable_nxe_bit();

    gdt::init();
    idt::init();
    dev::init_ap();

    crate::rust_main_ap();
}
