#![no_std]
#![no_main]
#![feature(llvm_asm)]
#![feature(lang_items)]
#![feature(thread_local)]

extern crate alloc;
extern crate rlibc;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;
extern crate user_alloc;

mod lang;

fn spawn_shell() {
    if let Ok(pid) = syscall::fork() {
        if pid == 0 {
            let tty = syscall::open("/dev/tty", syscall_defs::OpenFlags::RDWR)
                .expect("Failed to open tty");

            syscall::ioctl(tty, syscall_defs::ioctl::tty::TIOCSCTTY, 0)
                .expect("Failed to attach terminal");

            if let Err(e) = syscall::exec("/bin/shell", None, None) {
                panic!("Failed to spawn shell {:?}", e);
            }

            unreachable!();
        }
    } else {
        panic!("init: fork failed");
    };
}

fn main() -> ! {
    spawn_shell();

    loop {
        if let Ok(r) = syscall::waitpid(0) {
            println!("init: child terminated: {}", r);
        }
    }
}
