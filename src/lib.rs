#![no_std]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]
#![feature(asm)]

extern crate rlibc;
extern crate x86;
extern crate spin;
#[macro_use]
extern crate lazy_static;

#[macro_use]
pub mod arch;
mod drivers;
pub mod lang_items;

#[no_mangle]
pub extern "C" fn rust_main() {
    println!("Hello World!");

    panic!("Oh!");
}
