pub mod cpuio;

#[macro_use]
pub mod output;

pub mod raw;
mod gdt;
mod idt;

#[no_mangle]
pub extern "C" fn x86_64_rust_main() {
    output::clear();
    gdt::init();
    idt::init();


    println!("Hello Arch!");
    ::rust_main();
}