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
pub mod ipi;
pub mod mm;
pub mod signal;
pub mod smp;
pub mod syscall;
pub mod time;
pub mod timer;
pub mod tls;
pub mod utils;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: mm::PhysAddr, stack_top: VirtAddr) {
    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    output::init(mboot.framebuffer_info_tag());
    let fb_info = mboot.framebuffer_info_tag().unwrap();

    logln!(
        "fb addr {:x} - {:x}",
        fb_info.addr(),
        fb_info.addr() + (fb_info.height() * fb_info.pitch()) as u64
    );
    logln!(
        "fb addr {} - {} x {}",
        fb_info.width(),
        fb_info.height(),
        fb_info.pitch()
    );

    gdt::early_init();

    println!("[ OK ] GDT Initialised");

    idt::init();

    println!("[ OK ] IDT Initialised");

    mm::init(&mboot);

    crate::kernel::mm::init();

    println!("[ OK ] Heap Initialised");

    mm::phys::init_pages();

    println!("[ OK ] Phys Page Map Initialised");

    if let Some(mo) = mboot.command_line_tag() {
        crate::kernel::params::init(mo.command_line());
    }

    output::debug::init();

    acpi::init();

    dev::init();

    crate::rust_main(stack_top);
}

#[no_mangle]
pub extern "C" fn x86_64_rust_main_ap() {
    crate::arch::raw::mm::enable_nxe_bit();

    gdt::early_init();

    idt::init_ap();

    dev::init_ap();

    let trampoline = crate::arch::smp::Trampoline::get();
    crate::rust_main_ap(trampoline.stack_ptr, trampoline.cpu_num);
}
