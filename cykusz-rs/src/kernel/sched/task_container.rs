use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;

use crate::kernel::mm::MappedAddr;
use crate::kernel::sync::Spin;
use crate::kernel::task::Task;

pub struct TaskContainer {
    tasks: Spin<BTreeMap<usize, Arc<Task>>>,
}

impl Default for TaskContainer {
    fn default() -> TaskContainer {
        TaskContainer {
            tasks: Spin::new(BTreeMap::new()),
        }
    }
}

impl TaskContainer {
    pub fn add_task(&self, fun: fn()) -> Arc<Task> {
        let task = Arc::new(Task::new_kern(fun));

        self.register_task(task.clone());

        task
    }

    pub fn add_param_task(&self, fun: usize, val: usize) -> Arc<Task> {
        let task = Arc::new(Task::new_param_kern(fun, val));

        self.register_task(task.clone());

        task
    }

    pub fn add_user_task(&self, fun: MappedAddr, code_size: usize) -> Arc<Task> {
        let task = Arc::new(Task::new_user(fun, code_size));

        self.register_task(task.clone());

        task
    }

    pub fn remove_task(&self, id: usize) {
        self.tasks.lock().remove(&id);
    }

    pub fn register_task(&self, task: Arc<Task>) {
        self.tasks.lock().insert(task.id(), task);
    }
}
