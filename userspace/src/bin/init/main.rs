#![no_std]
#![no_main]

extern crate program;
extern crate syscall_defs;
#[macro_use]
extern crate syscall_user as syscall;

fn spawn_shell() {
    if let Ok(pid) = syscall::fork() {
        if pid == 0 {
            if let Err(e) = syscall::setsid() {
                println!("[ init ] setsid failed {:?}", e);
            }
            if let Err(e) = syscall::exec("/bin/shell", None, None) {
                panic!("Failed to spawn shell {:?}", e);
            }

            unreachable!();
        }
    } else {
        panic!("init: fork failed");
    };
}

#[no_mangle]
pub fn main() {
    spawn_shell();

    let mut status = 0u32;

    loop {
        if let Ok(r) = syscall::waitpid(0, &mut status) {
            println!("[ init ]: child terminated: {}, status: {:#x}", r, status);
        }
    }
}
