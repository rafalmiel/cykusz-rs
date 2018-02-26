pub mod cpuio;

#[macro_use]
pub mod output;

mod gdt;

#[no_mangle]
pub extern "C" fn x86_64_rust_main() {
    ::arch::output::clear();
    gdt::init();
    println!("Hello Arch!");

    ::rust_main();
}