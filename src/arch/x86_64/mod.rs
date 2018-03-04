use drivers::multiboot2;

pub mod cpuio;

#[macro_use]
pub mod output;

#[macro_use]
pub mod raw;
pub mod gdt;
pub mod idt;
pub mod types;
pub mod mm;

#[no_mangle]
pub extern "C" fn x86_64_rust_main(mboot_addr: types::PhysAddr) {
    output::clear();
    gdt::init();
    idt::init();

    let mboot = unsafe { multiboot2::load(mboot_addr.to_mapped()) };

    mm::init(&mboot);

    let f1 = mm::phys::allocate().unwrap();
    let f2 = mm::phys::allocate().unwrap();
    let f3 = mm::phys::allocate().unwrap();
    let f4 = mm::phys::allocate().unwrap();
    let f5 = mm::phys::allocate().unwrap();

    println!("Alloc phys: {}", f1.address());
    println!("Alloc phys: {}", f2.address());
    println!("Alloc phys: {}", f3.address());
    println!("Alloc phys: {}", f4.address());
    println!("Alloc phys: {}", f5.address());

    mm::phys::deallocate(&f1);
    mm::phys::deallocate(&f3);
    mm::phys::deallocate(&f5);

    let f1 = mm::phys::allocate().unwrap();
    let f2 = mm::phys::allocate().unwrap();
    let f3 = mm::phys::allocate().unwrap();
    let f4 = mm::phys::allocate().unwrap();
    let f5 = mm::phys::allocate().unwrap();

    println!("Alloc phys: {}", f1.address());
    println!("Alloc phys: {}", f2.address());
    println!("Alloc phys: {}", f3.address());
    println!("Alloc phys: {}", f4.address());
    println!("Alloc phys: {}", f5.address());

    println!("Mboot test ks: {} ke: {}",mboot.kernel_start_addr(), mboot.kernel_end_addr());

    println!("Hello Arch!");
    ::rust_main();
}