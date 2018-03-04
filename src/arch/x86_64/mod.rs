use drivers::multiboot2;

#[macro_use]
pub mod output;

#[macro_use]
pub mod raw;
pub mod gdt;
pub mod idt;
pub mod mm;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: mm::PhysAddr) {
    output::clear();
    gdt::init();
    idt::init();

    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    mm::init(&mboot);

    ::rust_main();
}