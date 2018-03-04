pub mod cpuio;

#[macro_use]
pub mod output;

#[macro_use]
pub mod raw;
pub mod gdt;
pub mod idt;
pub mod types;

#[no_mangle]
pub extern "C" fn x86_64_rust_main() {
    output::clear();
    gdt::init();
    idt::init();

    println!("Hello Arch!");
    ::rust_main();
}