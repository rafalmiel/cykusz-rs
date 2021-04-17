use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::fmt::Debug;
use core::fmt::{Error, Formatter};

use input::*;
use syscall_defs::signal::{SIGHUP, SIGINT, SIGQUIT};
use syscall_defs::OpenFlags;

use crate::arch::output::{video, Color, ConsoleWriter};
use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::kbd::keys::KeyCode;
use crate::kernel::kbd::KeyListener;
use crate::kernel::sched::{current_task, current_task_ref};
use crate::kernel::session::{sessions, Group};
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::Spin;
use crate::kernel::syscall::sys::PollTable;
use crate::kernel::task::Task;
use crate::kernel::tty::TerminalDevice;
use crate::kernel::utils::wait_queue::WaitQueue;

use self::output::OutputBuffer;

mod input;
mod keymap;
mod output;

struct State {
    lshift: bool,
    rshift: bool,
    caps: bool,
    lctrl: bool,
    rctrl: bool,
    alt: bool,
    altgr: bool,
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "({:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8})",
            self.lshift, self.rshift, self.caps, self.lctrl, self.rctrl, self.alt, self.altgr
        )
    }
}

struct Tty {
    dev_id: usize,
    state: Spin<State>,
    buffer: Spin<InputBuffer>,
    output: Spin<OutputBuffer>,
    wait_queue: WaitQueue,
    ctrl_task: Spin<Option<Arc<Task>>>,
    fg_group: Spin<Option<Arc<Group>>>,
    self_ptr: Weak<Tty>,
}

impl State {
    const fn new() -> State {
        State {
            lshift: false,
            rshift: false,
            caps: false,
            lctrl: false,
            rctrl: false,
            alt: false,
            altgr: false,
        }
    }

    fn map(&self, apply_caps: bool) -> Option<&'static [u16]> {
        let mut shift = self.lshift || self.rshift;
        let ctrl = self.lctrl || self.rctrl;
        let alt = self.alt;
        let altgr = self.altgr;

        if apply_caps && self.caps {
            shift = !shift;
        }

        match (shift, ctrl, alt, altgr) {
            (false, false, false, false) => Some(&keymap::PLAIN_MAP),
            (true, false, false, false) => Some(&keymap::SHIFT_MAP),
            (false, true, false, false) => Some(&keymap::CTRL_MAP),
            (false, false, true, false) => Some(&keymap::ALT_MAP),
            (false, false, false, true) => Some(&keymap::ALTGR_MAP),
            (true, true, false, false) => Some(&keymap::CTRL_SHIFT_MAP),
            (false, true, true, false) => Some(&keymap::CTRL_ALT_MAP),
            _ => None,
        }
    }
}

impl Tty {
    fn new() -> Arc<Tty> {
        let video = video();
        let (sx, sy) = video.dimensions();
        let output = OutputBuffer::new(sx, sy, 1000, Color::LightGreen, Color::Black);
        Arc::new_cyclic(|me| Tty {
            dev_id: crate::kernel::device::alloc_id(),
            state: Spin::new(State::new()),
            buffer: Spin::new(InputBuffer::new()),
            output: Spin::new(output),
            wait_queue: WaitQueue::new(),
            ctrl_task: Spin::new(None),
            fg_group: Spin::new(None),
            self_ptr: me.clone(),
        })
    }

    fn read(&self, buf: *mut u8, len: usize) -> SignalResult<usize> {
        let mut buffer = self
            .wait_queue
            .wait_lock_irq_for(&self.buffer, |lck| lck.has_data())?;

        Ok(buffer.read(buf, len))
    }
}

impl KeyListener for Tty {
    fn on_new_key(&self, key: KeyCode, released: bool) {
        //println!("new key begin");
        let mut state = self.state.lock();

        match key {
            KeyCode::KEY_CAPSLOCK if !released => {
                state.caps = !state.caps;
            }
            KeyCode::KEY_LEFTSHIFT => {
                state.lshift = !released;
            }
            KeyCode::KEY_RIGHTSHIFT => {
                state.rshift = !released;
            }
            KeyCode::KEY_LEFTCTRL => {
                state.lctrl = !released;
            }
            KeyCode::KEY_RIGHTCTRL => {
                state.rctrl = !released;
            }
            KeyCode::KEY_LEFTALT => {
                state.alt = !released;
            }
            KeyCode::KEY_RIGHTALT => {
                state.altgr = !released;
            }
            KeyCode::KEY_BACKSPACE if !released => {
                let n = self.buffer.lock().remove_last_n(1);
                if n > 0 {
                    self.output.lock().remove_last_n(n);
                }
            }
            KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER if !released => {
                {
                    let mut buf = self.buffer.lock();
                    buf.put_char('\n' as u8);
                    buf.commit_write();
                }
                self.output.lock().put_char(b'\n');
                if let Some(_t) = &*self.ctrl_task.lock_irq() {
                    self.wait_queue.notify_all();
                }
            }
            KeyCode::KEY_U if (state.lctrl || state.rctrl) && !released => {
                let n = self.buffer.lock().remove_all_edit();
                if n > 0 {
                    self.output.lock().remove_last_n(n);
                }
            }
            KeyCode::KEY_C if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.fg_group.lock().as_ref() {
                    t.for_each(&|t| {
                        t.signal(SIGINT);
                    })
                }
            }
            KeyCode::KEY_BACKSLASH if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.fg_group.lock().as_ref() {
                    t.for_each(&|t| {
                        t.signal(SIGQUIT);
                    })
                }
            }
            KeyCode::KEY_PAGEDOWN if !released => {
                self.output.lock().scroll_down(20);
            }
            KeyCode::KEY_PAGEUP if !released => {
                self.output.lock().scroll_up(20);
            }
            KeyCode::KEY_HOME if !released => {
                self.output.lock().scroll_top();
            }
            KeyCode::KEY_END if !released => {
                self.output.lock().scroll_bottom();
            }
            KeyCode::KEY_UP if !released => {
                self.output.lock().scroll_up(1);
            }
            KeyCode::KEY_DOWN if !released => {
                self.output.lock().scroll_down(1);
            }
            _ if !released => {
                if let Some(finalmap) = state.map(false).map_or(None, |map| {
                    match state.caps {
                        // 0xfb marker denotes letter than is
                        // affected by caps lock
                        true if (map[key as usize] >> 8) & 0xff == 0xfb => {
                            // Return map after applying capslock to current shift state
                            state.map(true)
                        }
                        _ => Some(map),
                    }
                }) {
                    let sym = ((finalmap[key as usize] & 0xff) as u8) as char;

                    self.buffer.lock().put_char(sym as u8);
                    self.output.lock().put_char(sym as u8);
                }
            }
            _ => {}
        };
    }
}

impl Device for Tty {
    fn id(&self) -> usize {
        self.dev_id
    }

    fn name(&self) -> String {
        String::from("tty")
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.self_ptr.upgrade().unwrap()
    }
}

impl TerminalDevice for Tty {
    fn id(&self) -> usize {
        self.dev_id
    }

    fn ctrl_process(&self) -> Option<Arc<Task>> {
        self.ctrl_task.lock().as_ref().cloned()
    }

    fn attach(&self, task: Arc<Task>) -> bool {
        if !task.is_session_leader() {
            return false;
        }

        let mut cur = self.ctrl_task.lock_irq();

        if cur.is_none() {
            println!("[ TTY ] Attached task {}", task.tid());
            if let Some(group) = sessions().get_group(task.sid(), task.gid()) {
                self.set_fg_group(group);
            }

            *cur = Some(task);

            return true;
        }

        false
    }

    fn detach(&self, task: Arc<Task>) -> bool {
        let mut ctrl = self.ctrl_task.lock_irq();

        if let Some(cur) = ctrl.as_ref() {
            if cur.pid() == task.pid() {
                println!("[ TTY ] Detached task {}", cur.tid());
                *ctrl = None;

                let mut group = self.fg_group.lock();

                if let Some(group) = &*group {
                    group.for_each(&|t| {
                        t.signal(SIGHUP);
                    })
                }

                *group = None;

                drop(group);
                drop(ctrl);

                if let Some(session) = sessions().get_session(task.sid()) {
                    session.for_each(|t| {
                        if t.pid() != task.pid() {
                            t.terminal().disconnect(None);
                        }
                    })
                }

                return true;
            }
        }

        false
    }

    fn set_fg_group(&self, group: Arc<Group>) -> bool {
        *self.fg_group.lock() = Some(group);

        true
    }
}

impl INode for Tty {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize, FsError> {
        Ok(self.read(buf.as_mut_ptr(), buf.len())?)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize, FsError> {
        if let Err(_) = self.write_str(unsafe { core::str::from_utf8_unchecked(buf) }) {
            Err(FsError::InvalidParam)
        } else {
            Ok(buf.len())
        }
    }

    fn poll(&self, ptable: Option<&mut PollTable>) -> Result<bool, FsError> {
        let has_data = self.buffer.lock().has_data();

        if let Some(p) = ptable {
            p.listen(&self.wait_queue);
        }

        Ok(has_data)
    }

    fn open(&self, flags: OpenFlags) -> Result<(), FsError> {
        let current_task = current_task_ref();

        let ctty = !flags.contains(OpenFlags::NOCTTY);

        if ctty {
            let ctrl = self.ctrl_task.lock_irq();

            if ctrl.is_none() && current_task.is_session_leader() {
                if current_task.terminal().terminal().is_none() {
                    drop(ctrl);

                    current_task.terminal().connect(tty().clone());
                }
            }
        }

        Ok(())
    }

    fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize, FsError> {
        match cmd {
            syscall_defs::ioctl::tty::TIOCSCTTY => {
                let current = current_task_ref();

                if !current.is_session_leader() {
                    return Err(FsError::NoPermission);
                }

                if current_task().terminal().connect(tty().clone()) {
                    Ok(0)
                } else {
                    Err(FsError::EntryExists)
                }
            }
            syscall_defs::ioctl::tty::TIOCNOTTY => {
                if current_task().terminal().disconnect(Some(tty().clone())) {
                    Ok(0)
                } else {
                    Err(FsError::EntryNotFound)
                }
            }
            syscall_defs::ioctl::tty::TIOCSPGRP => {
                let gid = arg;

                let task = current_task_ref();

                if !task.terminal().is_connected(tty().clone()) {
                    return Err(FsError::NoTty);
                }

                if let Some(ctrl) = self.ctrl_task.lock_irq().as_ref() {
                    if ctrl.sid() == task.sid() {
                        if let Some(group) = sessions().get_group(task.sid(), gid) {
                            self.set_fg_group(group);

                            return Ok(0);
                        }
                    }
                }

                Err(FsError::NoTty)
            }
            _ => Err(FsError::NotSupported),
        }
    }
}

impl ConsoleWriter for Tty {
    fn write_str(&self, s: &str) -> core::fmt::Result {
        self.output.lock().write_str(s);
        Ok(())
    }
}

lazy_static! {
    static ref TTY: Arc<Tty> = Tty::new();
}

fn tty() -> &'static Arc<Tty> {
    &TTY
}

fn init() {
    crate::kernel::kbd::register_key_listener(tty().as_ref());
    if let Err(v) = crate::kernel::device::register_device(tty().clone()) {
        panic!("Failed to register Tty device: {:?}", v);
    }
    if let Err(v) = crate::kernel::tty::register_tty(tty().clone()) {
        panic!("Failed to register Tty terminal {:?}", v);
    }
    crate::arch::output::register_output_driver(tty().as_ref());
    video().clear();
}

module_init!(init);
