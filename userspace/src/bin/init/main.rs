#![no_std]
#![no_main]

extern crate program;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

use syscall_defs::waitpid::WaitPidFlags;

fn spawn_shell() -> usize {
    if let Ok(pid) = syscall::fork() {
        if pid == 0 {
            if let Err(e) = syscall::setsid() {
                println!("[ init ] setsid failed {:?}", e);
            }
            syscall::ioctl(0, syscall_defs::ioctl::tty::TIOCSCTTY, 0)
                .expect("Failed to attach tty");
            syscall::setpgid(0, 0).expect("Failed to make process a group leader");
            if let Err(e) = syscall::exec(
                "/usr/bin/bash",
                None,
                Some(&["PATH=/bin:/usr/bin", "TERM=cykusz"]),
            ) {
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
        if let Ok(r) = syscall::waitpid(-1, &mut status, WaitPidFlags::EXITED) {
            println!(
                "[ init ]: child terminated: {}, status: {:?}",
                r,
                syscall_defs::waitpid::Status::from(status)
            );

            if r == shell_pid {
                shell_pid = spawn_shell();
            }
        }
    }
}
