#![no_std]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]

extern crate rlibc;
extern crate x86;
extern crate spin;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod arch;
mod drivers;
pub mod lang_items;

pub fn clear() {
    ::arch::output::clear();
}

#[no_mangle]
pub extern "C" fn rust_main() {

    clear();
    println!("Hello World!");

    panic!("Oh!");
}
