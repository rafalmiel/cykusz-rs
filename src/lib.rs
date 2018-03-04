#![no_std]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(concat_idents)]
#![feature(step_trait)]
#![feature(iterator_step_by)]


extern crate rlibc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate spin;

#[macro_use]
pub mod arch;
mod drivers;
pub mod kernel;
pub mod lang_items;

#[no_mangle]
pub extern "C" fn rust_main() {
    println!("Hello World!");

    panic!("Oh!");
}
