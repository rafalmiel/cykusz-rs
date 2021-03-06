use alloc::sync::Arc;

use crate::kernel::sync::Spin;
use crate::kernel::task::Task;

pub struct TaskContainer {
    tasks: Spin<hashbrown::HashMap<usize, Arc<Task>>>,
}

impl Default for TaskContainer {
    fn default() -> TaskContainer {
        TaskContainer {
            tasks: Spin::new(hashbrown::HashMap::new()),
        }
    }
}

impl TaskContainer {
    #[allow(unused)]
    pub fn get(&self, id: usize) -> Option<Arc<Task>> {
        self.tasks.lock().get(&id).cloned()
    }

    pub fn remove_task(&self, id: usize) {
        self.tasks.lock().remove(&id).expect("not found");
    }

    pub fn register_task(&self, task: Arc<Task>) {
        self.tasks.lock().insert(task.id(), task);
    }

    pub fn close_all_tasks(&self) {
        let tasks = self.tasks.lock();

        for (_, t) in tasks.iter() {
            t.clear_cwd();
            t.vm().clear();
            t.close_all_files();
        }
    }
}
