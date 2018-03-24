use drivers::multiboot2;

#[macro_use]
pub mod output;
#[macro_use]
pub mod raw;
pub mod gdt;
pub mod idt;
pub mod mm;
pub mod acpi;
pub mod int;
pub mod dev;
pub mod sync;
pub mod smp;
pub mod tls;

use kernel::mm::PhysAddr;

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

    ::rust_main();
}

#[no_mangle]
pub extern "C" fn x86_64_rust_main_ap() {
    ::arch::raw::mm::enable_nxe_bit();

    gdt::init();
    idt::init();
    dev::init_ap();

    ::rust_main_ap();
}
