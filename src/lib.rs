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
#![feature(integer_atomics)]

extern crate rlibc;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate linked_list_allocator;
extern crate raw_cpuid;

use kernel::mm::{VirtAddr, MappedAddr};

use core::sync::atomic::{AtomicBool, AtomicU64, ATOMIC_BOOL_INIT, ATOMIC_U64_INIT, Ordering};

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
static USER_PROGRAM: AtomicU64 = ATOMIC_U64_INIT;
static USER_PROGRAM_SIZE: AtomicU64 = ATOMIC_U64_INIT;

pub fn bochs() {
    unsafe {
        asm!("xchg %bx, %bx");
    }
}

pub fn rust_main(stack_top: VirtAddr, user_program: Option<(MappedAddr, usize)>) {
    if let Some(addr) = user_program {
        USER_PROGRAM.store((addr.0).0 as u64, Ordering::SeqCst);
        USER_PROGRAM_SIZE.store(addr.1 as u64, Ordering::SeqCst);
    }

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

    kernel::timer::start();

    println!("[ OK ] Local Timer Started");

    // Start test tasks on this cpu
    if let Some(addr) = user_program {
        task_test::start(addr.0, addr.1);
    }

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

    kernel::timer::start();

    let user_program = USER_PROGRAM.load(Ordering::SeqCst);
    let user_program_size = USER_PROGRAM_SIZE.load(Ordering::SeqCst);

    if user_program != 0 {
        // Start test tasks on this cpu
        task_test::start(MappedAddr(user_program as usize), user_program_size as usize);
    }

    loop {
        ::kernel::int::disable();
        if ::kernel::sched::reschedule() {
            ::kernel::int::enable();
        } else {
            ::kernel::int::enable_and_halt();
        }
    }
}
