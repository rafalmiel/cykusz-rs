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

use kernel::mm::PhysAddr;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: mm::PhysAddr) {
    output::clear();
    gdt::init();
    idt::init();

    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    mm::init(&mboot);

    acpi::init();

    dev::init();

    int::sti();

    println!("[ OK ] Interrupts enabled");

    ::rust_main();
}

#[no_mangle]
pub extern "C" fn x86_64_rust_main_ap() {
    ::arch::raw::mm::enable_nxe_bit();

    gdt::init();
    idt::init();
    int::sti();

    let cpu = unsafe {
        PhysAddr(0xE01).to_mapped().read::<u8>()
    };
    unsafe {
        PhysAddr(0xE00).to_mapped().store::<u8>(1);
    }

    println!("[ OK ] Hello from cpu {}", cpu);

    loop {
        unsafe {
            asm!("pause"::::"volatile");
        }
    }
}
