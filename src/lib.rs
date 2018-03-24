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
#![feature(thread_local)]

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

#[thread_local]
static mut CPU_ID: u8 = 0;

pub fn rust_main() {
    kernel::mm::init();

    kernel::tls::init();

    println!("[ OK ] Per-CPU Storage Initialised");

    unsafe {
        CPU_ID = 0;
    }

    kernel::smp::init();

    println!("[ OK ] SMP Initialized");

    kernel::timer::start_timer();

    println!("[ OK ] Local Timer Started");

    kernel::int::enable_ints();

    loop {
        unsafe {
            asm!("pause"::::"volatile");
        }
    }
}

pub fn rust_main_ap() {
    kernel::tls::init();
    let trampoline = ::arch::smp::Trampoline::get();

    unsafe {
        CPU_ID = trampoline.cpu_num;
    }
    trampoline.notify_ready();

    unsafe {
        println!("[ OK ] CPU {} Ready!", CPU_ID);
    }

    kernel::timer::start_timer();

    kernel::int::enable_ints();

    loop {
        unsafe {
            asm!("pause"::::"volatile");
        }
    }
}
