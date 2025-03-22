use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use spin::Once;

use crate::kernel::device::dev_t::DevId;
use crate::kernel::fs::path::Path;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::fs::{lookup_by_real_path, LookupMode};
use crate::kernel::session::Group;
use crate::kernel::sync::{LockApi, RwSpin, Spin};
use crate::kernel::task::{ArcTask, WeakTask};

pub trait TerminalDevice: Send + Sync {
    fn id(&self) -> DevId;
    fn ctrl_process(&self) -> Option<ArcTask>;
    fn attach(&self, task: ArcTask) -> bool;
    fn detach(&self, task: ArcTask) -> bool;
    fn set_fg_group(&self, group: Arc<Group>) -> bool;
}

static TTY_DEVS: RwSpin<BTreeMap<DevId, Arc<dyn TerminalDevice>>> = RwSpin::new(BTreeMap::new());

pub fn register_tty(dev: Arc<dyn TerminalDevice>) -> crate::kernel::device::Result<()> {
    let mut devs = TTY_DEVS.write();

    println!("[ TTY ] Registered terminal device");

    devs.insert(dev.id(), dev);

    Ok(())
}

pub fn get_tty_by_id(id: DevId) -> Option<Arc<dyn TerminalDevice>> {
    let devs = TTY_DEVS.read();

    if let Some(d) = devs.get(&id) {
        Some(d.clone())
    } else {
        None
    }
}

pub struct Terminal {
    ctrl_term: Arc<Spin<Option<Arc<dyn TerminalDevice>>>>,
    proc: Once<WeakTask>,
}

impl Default for Terminal {
    fn default() -> Self {
        Terminal {
            ctrl_term: Default::default(),
            proc: Once::new(),
        }
    }
}

pub fn get_tty_by_path(path: &str) -> Result<Arc<dyn TerminalDevice>, FsError> {
    let entry = lookup_by_real_path(&Path::new(path), LookupMode::None)?;

    let device = entry.inode().device_id().ok_or(FsError::EntryNotFound)?;

    if let Some(tty) = get_tty_by_id(device) {
        Ok(tty)
    } else {
        Err(FsError::EntryNotFound)
    }
}

impl Terminal {
    pub fn init(&self, task: &WeakTask) {
        self.proc.call_once(|| task.clone());
    }

    fn task(&self) -> Option<ArcTask> {
        unsafe { self.proc.get_unchecked().upgrade() }
    }

    pub fn terminal(&self) -> Option<Arc<dyn TerminalDevice>> {
        self.ctrl_term.lock().as_ref().cloned()
    }

    pub fn is_connected(&self, to: Arc<dyn TerminalDevice>) -> bool {
        if let Some(t) = self.ctrl_term.lock().as_ref() {
            t.id() == to.id()
        } else {
            false
        }
    }

    pub fn connect(&self, terminal: Arc<dyn TerminalDevice>) -> bool {
        let task = self.task().expect("terminal: Task not set");

        logln3!("tty connect: pid: {}", task.tid());

        let mut term = self.ctrl_term.lock();

        if let Some(our) = term.as_ref() {
            return our.id() == terminal.id();
        } else {
            let is_leader = task.is_session_leader();

            if let Some(ctrl) = &terminal.ctrl_process() {
                if !is_leader && ctrl.sid() == task.sid() {
                    *term = Some(terminal);

                    return true;
                }

                false
            } else if is_leader {
                if !terminal.attach(task) {
                    return false;
                }

                *term = Some(terminal);

                true
            } else {
                false
            }
        }
    }

    pub fn disconnect(&self, terminal: Option<Arc<dyn TerminalDevice>>) -> bool {
        let task = self.task().expect("terminal: Task not set");

        let mut term = self.ctrl_term.lock();

        if let Some(t) = term.as_ref() {
            if let Some(target) = terminal {
                if target.id() != t.id() {
                    return false;
                }
            }

            let is_leader = task.is_session_leader();

            if !is_leader {
                *term = None;

                return true;
            } else {
                if let Some(ctrl) = t.ctrl_process() {
                    if ctrl.pid() == task.pid() && t.detach(task) {
                        *term = None;

                        return true;
                    }
                }

                false
            }
        } else {
            false
        }
    }

    pub fn share_with(&self, term: &mut Terminal) {
        term.ctrl_term = self.ctrl_term.clone();
    }
}

pub fn init() {
    crate::drivers::tty::init();
}
