#![no_std]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(concat_idents)]
#![feature(step_trait)]
#![feature(iterator_step_by)]
#![feature(alloc, allocator_api)]
#![feature(pointer_methods)]
#![feature(thread_local)]
#![feature(optin_builtin_traits)]

extern crate rlibc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate linked_list_allocator;
extern crate raw_cpuid;

use kernel::mm::VirtAddr;

use core::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};

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

static SMP_INITIALISED: AtomicBool = ATOMIC_BOOL_INIT;

pub fn bochs() {
    unsafe {
        asm!("xchg %bx, %bx");
    }
}

pub fn rust_main(stack_top: VirtAddr) {
    kernel::mm::init();

    kernel::tls::init(stack_top);

    println!("[ OK ] Per-CPU Storage Initialised");

    unsafe {
        CPU_ID = 0;
    }

    kernel::smp::init();

    println!("[ OK ] SMP Initialized (CPU count: {})", kernel::smp::cpu_count());

    SMP_INITIALISED.store(true, Ordering::SeqCst);

    kernel::sched::init();

    println!("[ OK ] Scheduler Initialised");

    kernel::timer::setup();

    println!("[ OK ] Local Timer Started");

    // Start test tasks on this cpu
    task_test::start();

    loop {
        ::kernel::int::disable();
        if ::kernel::sched::reschedule() {
            ::kernel::int::enable();
        } else {
            ::kernel::int::enable_and_halt();
        }
    }
}

pub fn rust_main_ap() {
    let trampoline = ::arch::smp::Trampoline::get();

    kernel::tls::init(VirtAddr(trampoline.stack_ptr as usize));

    unsafe {
        CPU_ID = trampoline.cpu_num;
    }

    trampoline.notify_ready();

    // Waiting for all CPUs to be ready
    while SMP_INITIALISED.load(Ordering::SeqCst) == false {
        unsafe {
            asm!("pause"::::"volatile");
        }
    }

    println!("[ OK ] CPU {} Initialised", unsafe {::CPU_ID});

    kernel::sched::init();

    kernel::timer::setup();

    // Start test tasks on this cpu
    task_test::start();

    loop {
        ::kernel::int::disable();
        if ::kernel::sched::reschedule() {
            ::kernel::int::enable();
        } else {
            ::kernel::int::enable_and_halt();
        }
    }
}
