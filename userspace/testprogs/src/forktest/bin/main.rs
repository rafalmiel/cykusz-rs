use syscall_defs::waitpid::WaitPidFlags;

fn main() {
    let mut test_var = 42;

    if let Ok(new_pid) = syscall_user::fork() {
        if new_pid == 0 {
            test_var = 12;
            println!("test_var after change: {}", test_var);
        } else {
            let mut status = 0;
            let estatus =
                syscall_user::waitpid(new_pid as isize, &mut status, WaitPidFlags::EXITED);
            println!(
                "child completed {:?} status: {:?}, test_var {}",
                estatus, status, test_var
            );
        }
    }
}
