use alloc::sync::Arc;

use crate::kernel::fs::dirent::DirEntryItem;
use crate::kernel::sched::current_task;
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
    pub fn add_task(&self, fun: fn()) -> Arc<Task> {
        let task = Arc::new(Task::new_kern(fun));

        self.register_task(task.clone());

        task
    }

    #[allow(unused)]
    pub fn get(&self, id: usize) -> Option<Arc<Task>> {
        self.tasks.lock().get(&id).cloned()
    }

    pub fn add_param_task(&self, fun: usize, val: usize) -> Arc<Task> {
        let task = Arc::new(Task::new_param_kern(fun, val));

        self.register_task(task.clone());

        task
    }

    pub fn add_user_task(&self, exe: DirEntryItem) -> Arc<Task> {
        let task = Arc::new(Task::new_user(exe));

        self.register_task(task.clone());

        task
    }

    pub fn fork(&self) -> Arc<Task> {
        let current = current_task();

        let task = Arc::new(current.fork());

        self.register_task(task.clone());

        task
    }

    pub fn exec(&self, exe: DirEntryItem) -> Arc<Task> {
        let current = current_task();

        let task = Arc::new(current.exec(exe));
        drop(current);

        self.register_task(task.clone());

        task
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
            t.close_all_files();
        }
    }
}
