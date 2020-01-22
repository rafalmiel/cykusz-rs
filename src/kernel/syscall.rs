use core::sync::atomic::Ordering;
use core::slice;
use core::str;

pub fn init() {
    crate::arch::syscall::init();
}

pub fn init_ap() {
    crate::arch::syscall::init_ap();
}

pub fn syscall_handler(num: u64, a: u64, b: u64) -> u64 {
    match num {
        0 => { // print syscall
            let ptr = a as *const u8;
            let len = b as usize;

            let s = unsafe {
                slice::from_raw_parts(ptr, len)
            };

            let s = str::from_utf8(s);

            match s {
                Ok(v) => {
                    print!("{}", v);
                },
                Err(_) => {
                    println!("Failed to obtain str");
                }
            }

            return 0;
        },
        1 => { // read stdin
            let n = crate::drivers::input::tty::read(a as *mut u8, b as usize);

            return n as u64;
        },
        2 => { // print diagnostics
            println!(
                "U( {:<6} PID: {:<6} CPU: {:<6} MEM: {:<8}{:<12}),",
                "",
                crate::kernel::sched::current_id(),
                unsafe { crate::CPU_ID },
                crate::kernel::mm::heap::ALLOCED_MEM.load(Ordering::SeqCst),
                ""
            );

            return 0;

        }
        _ => {
            return 0;
        }
    }
}
