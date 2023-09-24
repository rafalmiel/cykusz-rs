#![no_std]
#![no_main]

extern crate program;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

fn spawn_shell() -> usize {
    if let Ok(pid) = syscall::fork() {
        if pid == 0 {
            if let Err(e) = syscall::setsid() {
                println!("[ init ] setsid failed {:?}", e);
            }
            syscall::ioctl(0, syscall_defs::ioctl::tty::TIOCSCTTY, 0)
                .expect("Failed to attach tty");
            if let Err(e) = syscall::exec("/usr/bin/bash", None, None) {
                panic!("Failed to spawn shell {:?}", e);
            }

            unreachable!();
        } else {
            return pid;
        }
    } else {
        panic!("init: fork failed");
    };
}

#[no_mangle]
pub fn main() {
    let mut shell_pid = spawn_shell();

    let mut status = 0u32;

    loop {
        if let Ok(r) = syscall::waitpid(0, &mut status) {
            println!("[ init ]: child terminated: {}, status: {:#x}", r, status);

            if r == shell_pid {
                shell_pid = spawn_shell();
            }
        }
    }
}
