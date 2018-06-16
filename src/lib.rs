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

extern crate rlibc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate spin;
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

static mut TASK1: arch::task::Task = arch::task::Task::empty();
static mut TASK2: arch::task::Task = arch::task::Task::empty();

pub fn task_1() {
    loop {
        print!("1");

        unsafe {
            arch::task::switch(&mut TASK1, &TASK2);
        }
    }
}

pub fn task_2() {
    loop {
        print!("2");

        unsafe {
            arch::task::switch(&mut TASK2, &TASK1);
        }
    }
}

pub fn rust_main() {
    kernel::mm::init();

    kernel::tls::init();

    println!("[ OK ] Per-CPU Storage Initialised");

    unsafe {
        CPU_ID = 0;
    }

    kernel::smp::init();

    println!("[ OK ] SMP Initialized");

    kernel::timer::setup_timer();

    println!("[ OK ] Local Timer Started");

    //kernel::int::enable_ints();

    unsafe {
        TASK1 = arch::task::Task::new_kern(task_1);
        TASK2 = arch::task::Task::new_kern(task_2);
    }

    let mut t0 = arch::task::Task::empty();

    unsafe {
        arch::task::switch(&mut t0, &TASK1);
    }


    loop {
        unsafe {
            asm!("hlt"::::"volatile");
        }
    }
}

pub fn rust_main_ap() {
    kernel::tls::init();

    let trampoline = ::arch::smp::Trampoline::get();

    unsafe {
        CPU_ID = trampoline.cpu_num;
    }

    unsafe {
        println!("[ OK ] CPU {} Ready!", CPU_ID);
    }

    trampoline.notify_ready();

    kernel::timer::start_timer();

    //kernel::int::enable_ints();

    loop {
        unsafe {
            asm!("hlt"::::"volatile");
        }
    }
}
