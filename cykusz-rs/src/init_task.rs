use syscall_defs::OpenFlags;

use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_real_path, root_dentry, LookupMode};
use crate::kernel::sched::current_task_ref;
use crate::kernel::tty::get_tty_by_path;

pub fn exec() -> ! {
    let task = current_task_ref();

    task.set_cwd(root_dentry().unwrap().clone());

    task.open_file(
        lookup_by_real_path(Path::new("/dev/stdin"), LookupMode::None).expect("stdin open failed"),
        OpenFlags::RDONLY,
    );

    let stdout = lookup_by_real_path(Path::new("/dev/stdout"), LookupMode::None)
        .expect("stdout open failed");

    task.open_file(stdout.clone(), OpenFlags::WRONLY);
    task.open_file(stdout, OpenFlags::WRONLY);

    let tty = get_tty_by_path("/dev/tty").expect("Tty not found");

    task.terminal().attach(tty);

    let shell =
        lookup_by_real_path(Path::new("/bin/shell"), LookupMode::None).expect("Shell not found");

    crate::kernel::sched::exec(shell)
}
