#![no_std]
#![feature(lang_items)]
#![feature(const_fn)]
#![feature(const_if_match)]
#![feature(ptr_internals)]
#![feature(llvm_asm)]
#![feature(abi_x86_interrupt)]
#![feature(concat_idents)]
#![feature(step_trait)]
#![feature(allocator_api)]
#![feature(thread_local)]
#![feature(optin_builtin_traits)]
#![feature(nll)]
#![feature(linkage)]
#![feature(c_variadic)]
#![feature(maybe_uninit_ref)]
#![feature(negative_impls)]
#![feature(slice_fill)]

extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;

use crate::kernel::mm::VirtAddr;

#[global_allocator]
static mut HEAP: kernel::mm::heap::LockedHeap = kernel::mm::heap::LockedHeap::empty();

#[macro_use]
pub mod arch;
#[macro_use]
pub mod kernel;
mod drivers;
mod externs;
pub mod lang_items;
pub mod task_test;

#[thread_local]
static mut CPU_ID: u8 = 0;

pub fn bochs() {
    unsafe {
        llvm_asm!("xchg %bx, %bx");
    }
}

pub fn rust_main(stack_top: VirtAddr) {
    kernel::mm::init();

    println!("[ OK ] Heap Initialised");

    arch::mm::phys::init_pages();

    println!("[ OK ] Phys Page Map Initialised");

    kernel::smp::init();

    kernel::tls::init(stack_top);

    println!("[ OK ] Per-CPU Storage Initialised");

    unsafe {
        CPU_ID = 0;
    }

    kernel::fs::init();

    println!("[ OK ] VFS Initialized");

    kernel::sched::init();

    println!("[ OK ] Scheduler Initialised");

    kernel::smp::start();

    println!(
        "[ OK ] SMP Initialized (CPU count: {})",
        kernel::smp::cpu_count()
    );

    kernel::syscall::init();

    println!("[ OK ] Syscall Initialized");

    kernel::timer::setup();

    kernel::timer::start();

    println!("[ OK ] Local Timer Started");

    kernel::net::init();

    println!("[ OK ] Network Stack Initialized");

    kernel::module::init_all();

    println!("[ OK ] Modules Initialized");

    drivers::post_module_init();

    // Start test tasks on this cpu
    task_test::start();

    idle();
}

pub fn rust_main_ap(stack_ptr: u64, cpu_num: u8) {
    kernel::tls::init(VirtAddr(stack_ptr as usize));

    unsafe {
        crate::CPU_ID = cpu_num;
    }

    kernel::sched::enable_lock_protection();

    println!("[ OK ] CPU {} Initialised", unsafe { crate::CPU_ID });

    kernel::smp::notify_ap_ready();

    kernel::syscall::init_ap();

    kernel::timer::setup();

    kernel::timer::start();

    // Start test tasks on this cpu
    //task_test::start();

    idle();
}

fn idle() {
    loop {
        crate::kernel::int::disable();
        if crate::kernel::sched::reschedule() {
            crate::kernel::int::enable();
        } else {
            crate::kernel::int::enable_and_halt();
        }
    }
}
