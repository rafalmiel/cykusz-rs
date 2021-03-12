#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(arc_new_cyclic)]
#![feature(array_methods)]
#![feature(auto_traits)]
#![feature(c_variadic)]
#![feature(concat_idents)]
#![feature(const_btree_new)]
#![feature(const_fn)]
#![feature(const_mut_refs)]
#![feature(lang_items)]
#![feature(linkage)]
#![feature(llvm_asm)]
#![feature(maybe_uninit_ref)]
#![feature(negative_impls)]
#![feature(nll)]
#![feature(ptr_internals)]
#![feature(step_trait)]
#![feature(step_trait_ext)]
#![feature(thread_local)]
#![feature(try_blocks)]
#![feature(try_trait)]
#![feature(new_uninit)]
#![feature(linked_list_cursors)]

extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate intrusive_collections;
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
//mod externs;
pub mod init_task;
pub mod lang_items;

#[thread_local]
static mut CPU_ID: u8 = 0;

static mut DEBUG: bool = false;

pub fn enable_debug() {
    unsafe {
        DEBUG = true;
    }
}

pub fn disable_debug() {
    unsafe {
        DEBUG = false;
    }
}

pub fn is_debug() -> bool {
    unsafe { DEBUG }
}

pub fn bochs() {
    unsafe {
        llvm_asm!("xchg %bx, %bx");
    }
}

pub fn rust_main(stack_top: VirtAddr) {
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

    kernel::ipi::init();

    println!("[ OK ] IPI Initialized");

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

    let current = crate::kernel::sched::create_task(init_task);

    drop(current);

    idle();
}

fn init_task() {
    kernel::ipi::test_ipi();

    kernel::net::init();

    println!("[ OK ] Network Stack Initialized");

    kernel::module::init_all();

    println!("[ OK ] Modules Initialized");

    drivers::post_module_init();

    kernel::block::init();

    kernel::fs::mount_root();

    kernel::net::init();

    // Start test tasks on this cpu
    init_task::exec()
}

pub fn rust_main_ap(stack_ptr: u64, cpu_num: u8) {
    kernel::tls::init(VirtAddr(stack_ptr as usize));

    unsafe {
        crate::CPU_ID = cpu_num;
    }

    kernel::sched::init_ap();

    kernel::ipi::init_ap();

    println!("[ OK ] CPU {} Initialised", unsafe { crate::CPU_ID });

    kernel::syscall::init_ap();

    kernel::timer::setup();

    kernel::timer::start();

    kernel::smp::notify_ap_ready();

    idle();
}

fn idle() -> ! {
    loop {
        crate::kernel::int::disable();
        if crate::kernel::sched::reschedule() {
            crate::kernel::int::enable();
        } else {
            crate::kernel::int::enable_and_halt();
        }
    }
}
