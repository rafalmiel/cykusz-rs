use crate::kernel::fs::path::Path;
use crate::kernel::fs::{lookup_by_real_path, root_dentry, LookupMode};
use crate::kernel::sched::current_task_ref;

use crate::kernel::task::Task;
use alloc::sync::Arc;
use spin::Once;

static INIT: Once<Arc<Task>> = Once::new();

pub fn init_task() -> &'static Arc<Task> {
    unsafe { INIT.get_unchecked() }
}

pub fn exec() -> ! {
    let task = current_task_ref();

    INIT.call_once(|| task.clone());

    task.set_cwd(root_dentry().unwrap().clone());

    let init =
        lookup_by_real_path(Path::new("/bin/init"), LookupMode::None).expect("Shell not found");

    crate::kernel::sched::exec(init, None, None)
}
