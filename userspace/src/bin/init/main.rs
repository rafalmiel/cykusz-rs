use userspace as syscall;

use syscall_defs::waitpid::WaitPidFlags;

fn spawn_shell() -> usize {
    syscall::ioctl(0, syscall_defs::ioctl::tty::TIOCSCTTY, 0).expect("Failed to connect to tty");
    if let Ok(pid) = syscall::fork() {
        if pid == 0 {
            syscall::setpgid(0, 0).expect("Failed to make process a group leader");

            let pid = syscall::getpid().unwrap();
            syscall::ioctl(
                0,
                syscall_defs::ioctl::tty::TIOCSPGRP,
                core::ptr::addr_of!(pid) as usize,
            )
                .expect("Failed to attach tty");

            if let Err(e) = syscall::exec(
                "/usr/bin/sh",
                None,
                Some(&["PATH=/bin:/usr/bin", "TERM=cykusz"]),
            ) {
                panic!("Failed to spawn shell {:?}", e);
            }

            unreachable!();
        } else {
            syscall::setpgid(pid, pid).expect("Failed to make process a group leader");
            return pid;
        }
    } else {
        panic!("init: fork failed");
    };
}

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
