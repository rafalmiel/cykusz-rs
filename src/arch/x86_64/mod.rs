use drivers::multiboot2;
use kernel::mm::VirtAddr;

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
pub mod syscall;
pub mod user;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: mm::PhysAddr, stack_top: VirtAddr) {
    output::clear();

    gdt::early_init();

    println!("[ OK ] GDT Initialised");

    idt::init();
    println!("[ OK ] IDT Initialised");

    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    mm::init(&mboot);

    acpi::init();

    dev::init();

    //user::init(&mboot);

    ::rust_main(stack_top, user::find_user_program(mboot));
}

#[no_mangle]
pub extern "C" fn x86_64_rust_main_ap() {
    ::arch::raw::mm::enable_nxe_bit();

    gdt::early_init();
    idt::init();

    dev::init_ap();

    ::rust_main_ap();
}
