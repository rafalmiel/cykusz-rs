#![no_std]
#![allow(internal_features)]
#![feature(alloc_error_handler)]
#![feature(auto_traits)]
#![feature(c_variadic)]
#![feature(concat_idents)]
#![feature(lang_items)]
#![feature(linkage)]
#![feature(negative_impls)]
#![feature(ptr_internals)]
#![feature(step_trait)]
#![feature(thread_local)]
#![feature(try_blocks)]
#![feature(never_type)]
#![feature(linked_list_cursors)]
#![feature(trace_macros)]
#![feature(unsigned_is_multiple_of)]
#![feature(ptr_as_ref_unchecked)]
extern crate alloc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate intrusive_collections;
#[macro_use]
extern crate lazy_static;

use crate::arch::int;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_path, LookupMode};
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::current_task_ref;
use alloc::sync::Arc;
use core::any::Any;
use core::arch::asm;
use syscall_defs::OpenFlags;

#[global_allocator]
static mut HEAP: kernel::mm::heap::LockedHeap = kernel::mm::heap::LockedHeap::empty();

#[macro_use]
pub mod arch;
#[macro_use]
pub mod kernel;
mod drivers;
//mod externs;
pub mod lang_items;

#[thread_local]
static mut CPU_ID: u8 = 0;

static mut DEBUG: bool = false;

pub fn enable_debug() {
    unsafe {
        (&raw mut DEBUG).write(true);
    }
}

pub fn disable_debug() {
    unsafe {
        (&raw mut DEBUG).write(false);
    }
}

pub fn is_debug() -> bool {
    unsafe { DEBUG }
}

pub fn bochs() {
    unsafe {
        asm!("xchg bx, bx");
    }
}

pub fn cpu_id() -> u8 {
    unsafe { CPU_ID }
}

pub fn rust_main(stack_top: VirtAddr) {
    kernel::smp::init();

    kernel::tls::init(stack_top);

    println!("[ OK ] Per-CPU Storage Initialized");

    unsafe {
        (&raw mut CPU_ID).write(0);
    }

    kernel::fs::init();

    println!("[ OK ] VFS Initialized");

    kernel::session::init();

    println!("[ OK ] Sessions Initialized");

    kernel::sched::init();

    println!("[ OK ] Scheduler Initialized");

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

    kernel::module::init_all();

    println!("[ OK ] Modules Initialized");

    drivers::post_module_init();

    kernel::net::init();

    println!("[ OK ] Network Stack Initialized");

    kernel::block::init();

    kernel::fs::mount_root();

    kernel::tty::init();

    kernel::futex::init();

    println!("[ OK ] Futexes Initialized");

    current_task_ref()
        .open_file(
            lookup_by_path(&Path::new("/dev/tty"), LookupMode::None).unwrap(),
            OpenFlags::RDONLY,
        )
        .expect("Failed to open tty");
    current_task_ref()
        .open_file(
            lookup_by_path(&Path::new("/dev/tty"), LookupMode::None).unwrap(),
            OpenFlags::WRONLY,
        )
        .expect("Failed to open tty");
    current_task_ref()
        .open_file(
            lookup_by_path(&Path::new("/dev/tty"), LookupMode::None).unwrap(),
            OpenFlags::WRONLY,
        )
        .expect("Failed to open tty");

    // Start shell on this cpu
    crate::kernel::init::exec();
}

pub fn rust_main_ap(stack_ptr: u64, cpu_num: u8) {
    kernel::tls::init(VirtAddr(stack_ptr as usize));

    unsafe {
        (&raw mut CPU_ID).write(cpu_num);
    }

    kernel::sched::init_ap();

    kernel::ipi::init_ap();

    println!("[ OK ] CPU {} Initialized", unsafe { crate::CPU_ID });

    kernel::syscall::init_ap();

    kernel::timer::setup();

    kernel::timer::start();

    kernel::smp::notify_ap_ready();

    idle();
}

fn idle() -> ! {
    logln!(
        "entering idle task {}, locks {}",
        current_task_ref().tid(),
        current_task_ref().locks()
    );
    loop {
        kernel::int::disable();
        if kernel::sched::reschedule() {
            kernel::int::enable();
        } else {
            kernel::int::enable_and_halt();
        }
    }
}

fn sigexec_poweroff(_param: Arc<dyn Any + Send + Sync>) {
    // executing as init task here
    dbgln!(
        poweroff,
        "sigexec_poweroff, int enabled? {}",
        int::is_enabled()
    );
    kernel::sched::close_all_tasks();
    println!("[ SHUTDOWN ] Closed all tasks");
    kernel::fs::dirent::cache().clear();
    println!("[ SHUTDOWN ] Cleared dir cache");
    kernel::fs::icache::cache().clear();
    println!("[ SHUTDOWN ] Cleared inode cache");
    kernel::fs::mount::umount_all();
    println!("[ SHUTDOWN ] Unmounted fs");

    arch::acpi::power_off()
}
