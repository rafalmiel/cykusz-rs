use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;

use kernel::sync::Mutex;
use kernel::task::Task;
use kernel::mm::MappedAddr;

pub struct TaskContainer {
    tasks: Mutex<BTreeMap<usize, Arc<Task>>>
}

impl Default for TaskContainer {
    fn default() -> TaskContainer {
        TaskContainer {
            tasks: Mutex::new(BTreeMap::new())
        }
    }
}


impl TaskContainer {
    pub fn add_task(&self, fun: fn()) -> Arc<Task> {
        let task = Arc::new(Task::new_kern(fun));

        self.tasks.lock().insert(task.id(), task.clone());

        task
    }

    pub fn add_user_task(&self, fun: MappedAddr, code_size: usize, stack: usize) -> Arc<Task> {
        let task = Arc::new(Task::new_user(fun, code_size, stack));

        self.tasks.lock().insert(task.id(), task.clone());

        task
    }

    pub fn remove_task(&self, id: usize) {
        self.tasks.lock().remove(&id);
    }
}
