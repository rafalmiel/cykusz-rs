#![no_std]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(concat_idents)]
#![feature(step_trait)]
#![feature(iterator_step_by)]
#![feature(global_allocator)]
#![feature(alloc, allocator_api)]
#![feature(conservative_impl_trait)]
#![feature(pointer_methods)]
#![feature(dyn_trait)]

extern crate rlibc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate alloc;
extern crate linked_list_allocator;
extern crate raw_cpuid;

#[global_allocator]
static mut HEAP: kernel::mm::heap::LockedHeap = kernel::mm::heap::LockedHeap::empty();

#[macro_use]
pub mod arch;
mod drivers;
pub mod kernel;
pub mod lang_items;

#[no_mangle]
pub extern "C" fn rust_main() {
    ::kernel::mm::init();

    ::arch::smp::init();

    loop {
        unsafe {
            asm!("pause"::::"volatile");
        }
    }
}
