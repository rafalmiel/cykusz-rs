#![no_std]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(concat_idents)]
#![feature(step_trait)]
#![feature(alloc, allocator_api)]
#![feature(thread_local)]
#![feature(optin_builtin_traits)]
#![feature(const_vec_new)]
#![feature(nll)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate linked_list_allocator;
extern crate raw_cpuid;
extern crate rlibc;
extern crate spin;
extern crate alloc;

use kernel::mm::VirtAddr;
use alloc::sync::Arc;

#[global_allocator]
static mut HEAP: kernel::mm::heap::LockedHeap = kernel::mm::heap::LockedHeap::empty();

#[macro_use]
pub mod arch;
mod drivers;
#[macro_use]
pub mod kernel;
pub mod lang_items;
pub mod task_test;

#[thread_local]
static mut CPU_ID: u8 = 0;

pub fn bochs() {
    unsafe {
        asm!("xchg %bx, %bx");
    }
}

pub fn rust_main(stack_top: VirtAddr) {

    kernel::mm::init();

    kernel::smp::init();

    kernel::tls::init(stack_top);

    println!("[ OK ] Per-CPU Storage Initialised");

    unsafe {
        CPU_ID = 0;
    }

    kernel::sched::init();

    kernel::sched::enable_lock_protection();

    println!("[ OK ] Scheduler Initialised");

    kernel::smp::start();

    println!("[ OK ] SMP Initialized (CPU count: {})", kernel::smp::cpu_count());

    kernel::syscall::init();

    println!("[ OK ] Syscall Initialized");

    kernel::timer::setup();

    kernel::timer::start();

    println!("[ OK ] Local Timer Started");

    // Start test tasks on this cpu
    task_test::start();


    idle();
}

pub fn rust_main_ap(stack_ptr: u64, cpu_num: u8) {

    kernel::tls::init(VirtAddr(stack_ptr as usize));

    unsafe {
        ::CPU_ID = cpu_num;
    }

    kernel::sched::enable_lock_protection();

    println!("[ OK ] CPU {} Initialised", unsafe {::CPU_ID});

    kernel::smp::notify_ap_ready();

    kernel::timer::setup();

    kernel::timer::start();

    // Start test tasks on this cpu
    task_test::start();

    idle();
}

fn idle() {
    loop {
        ::kernel::int::disable();
        if ::kernel::sched::reschedule() {
            ::kernel::int::enable();
        } else {
            ::kernel::int::enable_and_halt();
        }
    }
}
