use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;

use crate::kernel::mm::MappedAddr;
use crate::kernel::sync::Mutex;
use crate::kernel::task::Task;

pub struct TaskContainer {
    tasks: Mutex<BTreeMap<usize, Arc<Task>>>,
}

impl Default for TaskContainer {
    fn default() -> TaskContainer {
        TaskContainer {
            tasks: Mutex::new(BTreeMap::new()),
        }
    }
}

impl TaskContainer {
    pub fn add_task(&self, fun: fn()) -> Arc<Task> {
        let task = Arc::new(Task::new_kern(fun));

        self.tasks.lock().insert(task.id(), task.clone());

        task
    }

    pub fn add_user_task(&self, fun: MappedAddr, code_size: usize) -> Arc<Task> {
        let task = Arc::new(Task::new_user(fun, code_size));

        self.tasks.lock().insert(task.id(), task.clone());

        task
    }

    pub fn remove_task(&self, id: usize) {
        self.tasks.lock().remove(&id);
    }

    pub fn get_task(&self, id: usize) -> Arc<Task> {
        self.tasks.lock().get(&id).expect("Task not found").clone()
    }
}
