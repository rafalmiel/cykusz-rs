use alloc::sync::{Arc, Weak};

use crate::kernel::fs::path::Path;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::fs::{lookup_by_real_path, LookupMode};
use crate::kernel::sync::{RwSpin, Spin};
use crate::kernel::task::Task;
use alloc::collections::BTreeMap;
use spin::Once;

pub trait TerminalDevice: Send + Sync {
    fn id(&self) -> usize;
    fn attach(&self, task: Arc<Task>) -> bool;
    fn detach(&self, task: Arc<Task>) -> bool;
}

static TTY_DEVS: RwSpin<BTreeMap<usize, Arc<dyn TerminalDevice>>> = RwSpin::new(BTreeMap::new());

pub fn register_tty(dev: Arc<dyn TerminalDevice>) -> crate::kernel::device::Result<()> {
    let mut devs = TTY_DEVS.write();

    println!("[ TTY ] Registered terminal device");

    devs.insert(dev.id(), dev);

    Ok(())
}

pub fn get_tty_by_id(id: usize) -> Option<Arc<dyn TerminalDevice>> {
    let devs = TTY_DEVS.read();

    if let Some(d) = devs.get(&id) {
        Some(d.clone())
    } else {
        None
    }
}

pub struct Terminal {
    attached_to: Spin<Option<Arc<dyn TerminalDevice>>>,
    proc: Once<Weak<Task>>,
}

impl Default for Terminal {
    fn default() -> Self {
        Terminal {
            attached_to: Default::default(),
            proc: Once::new(),
        }
    }
}

pub fn get_tty_by_path(path: &str) -> Result<Arc<dyn TerminalDevice>, FsError> {
    let entry = lookup_by_real_path(Path::new(path), LookupMode::None)?;

    let device = entry.inode().device()?;

    if let Some(tty) = get_tty_by_id(device.id()) {
        Ok(tty)
    } else {
        Err(FsError::EntryNotFound)
    }
}

impl Terminal {
    pub fn init(&self, task: &Weak<Task>) {
        self.proc.call_once(|| task.clone());
    }

    fn task(&self) -> Option<Arc<Task>> {
        self.proc.get().unwrap().upgrade()
    }

    pub fn detach(&self) -> bool {
        let task = self.task().expect("terminal: Task not set");

        let mut term = self.attached_to.lock();

        if let Some(t) = term.as_ref() {
            if t.detach(task) {
                *term = None;

                return true;
            }
        }

        false
    }

    pub fn detach_term(&self, terminal: Arc<dyn TerminalDevice>) -> bool {
        let task = self.task().expect("terminal: Task not set");

        let mut term = self.attached_to.lock();

        if let Some(t) = term.as_ref() {
            if t.id() == terminal.id() {
                if t.detach(task) {
                    *term = None;

                    return true;
                }
            }
        }

        false
    }

    pub fn attach(&self, terminal: Arc<dyn TerminalDevice>) -> bool {
        let task = self.task().expect("terminal: Task not set");

        let mut term = self.attached_to.lock();

        if term.is_none() {
            if terminal.attach(task) {
                *term = Some(terminal);

                return true;
            }
        }

        false
    }

    pub fn try_transfer_to(&self, task: Arc<Task>) {
        let mut term = self.attached_to.lock();

        if let Some(t) = term.as_ref() {
            if let Some(me) = self.task() {
                if t.detach(me) {
                    if task.terminal().attach(t.clone()) {
                        *term = None;
                    } else {
                        panic!("Failed to attach terminal to task {}", task.id());
                    }
                } else {
                    println!("detach failed");
                }
            } else {
                println!("no task found");
            }
        } else {
            println!("no terminal found");
        }
    }
}
