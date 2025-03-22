use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::task::ArcTask;

pub struct TaskContainer {
    tasks: Spin<hashbrown::HashMap<usize, ArcTask>>,
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
    pub fn get(&self, id: usize) -> Option<ArcTask> {
        self.tasks.lock().get(&id).cloned()
    }

    pub fn remove_task(&self, id: usize) {
        if let Some(_) = self.tasks.lock().remove(&id) {
            dbgln!(task, "task {} removed from container", id);
        }
    }

    pub fn register_task(&self, task: ArcTask) {
        if matches!(self.tasks.lock().insert(task.tid(), task.clone()), None) {
            dbgln!(task, "task {} registered in container", task.tid());
        }
    }

    pub fn close_all_tasks(&self) {
        let init = crate::kernel::init::init_task();

        init.terminate_children();

        let tasks = self.tasks.lock();

        for (_, t) in tasks.iter() {
            t.clear_cwd();
            t.vm().clear();
            t.close_all_files();
        }
    }
}
