use crate::drivers::multiboot2;
use crate::kernel::mm::VirtAddr;

#[macro_use]
pub mod output;
#[macro_use]
pub mod raw;
#[macro_use]
pub mod task;
pub mod acpi;
pub mod dev;
pub mod gdt;
pub mod idt;
pub mod int;
pub mod mm;
pub mod smp;
pub mod syscall;
pub mod time;
pub mod timer;
pub mod tls;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: mm::PhysAddr, stack_top: VirtAddr) {
    output::init();

    gdt::early_init();

    println!("[ OK ] GDT Initialised");

    idt::init();
    println!("[ OK ] IDT Initialised");

    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    mm::init(&mboot);

    crate::kernel::mm::init();

    println!("[ OK ] Heap Initialised");

    mm::phys::init_pages();

    println!("[ OK ] Phys Page Map Initialised");

    acpi::init();

    dev::init();

    crate::rust_main(stack_top);
}

#[no_mangle]
pub extern "C" fn x86_64_rust_main_ap() {
    crate::arch::raw::mm::enable_nxe_bit();

    gdt::early_init();
    idt::init();

    dev::init_ap();

    let trampoline = crate::arch::smp::Trampoline::get();
    crate::rust_main_ap(trampoline.stack_ptr, trampoline.cpu_num);
}
