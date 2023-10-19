use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::fmt::Debug;
use core::fmt::{Error, Formatter};

use input::*;
use syscall_defs::ioctl::tty;
use syscall_defs::poll::PollEventFlags;
use syscall_defs::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTSTP};
use syscall_defs::OpenFlags;

use crate::arch::output::{video, Color, ConsoleWriter};
use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::kbd::keys::KeyCode;
use crate::kernel::kbd::KeyListener;
use crate::kernel::mm::VirtAddr;
use crate::kernel::sched::current_task_ref;
use crate::kernel::session::{sessions, Group};
use crate::kernel::signal::{SignalError, SignalResult};
use crate::kernel::sync::{Spin, SpinGuard};
use crate::kernel::task::Task;

use crate::kernel::tty::TerminalDevice;
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

use self::output::OutputBuffer;

mod input;
mod keymap;
mod output;

const BACKLOG_SIZE: usize = 1000;

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
    termios: Spin<tty::Termios>,
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

    fn map_raw_sequence(&self, key_code: KeyCode) -> Option<&'static [u8]> {
        return keymap::RAW_MODE_MAP[key_code as usize];
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
            (true, false, _, false) => Some(&keymap::SHIFT_MAP),
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
        let output = OutputBuffer::new(sx, sy, BACKLOG_SIZE, Color::LightGreen, Color::Black);
        Arc::new_cyclic(|me| Tty {
            dev_id: crate::kernel::device::alloc_id(),
            state: Spin::new(State::new()),
            buffer: Spin::new(InputBuffer::new()),
            output: Spin::new(output),
            wait_queue: WaitQueue::new(),
            ctrl_task: Spin::new(None),
            fg_group: Spin::new(None),
            self_ptr: me.clone(),
            termios: Spin::new(tty::Termios::default()),
        })
    }

    fn read(&self, buf: *mut u8, len: usize) -> SignalResult<usize> {
        if let Some(fg) = &*self.fg_group.lock_irq() {
            let task = current_task_ref();

            if !fg.has_process(task.pid()) {
                task.signal(syscall_defs::signal::SIGTTIN);
                return Err(SignalError::Interrupted);
            }
        }
        let mut buffer = self
            .wait_queue
            .wait_lock_for(WaitQueueFlags::IRQ_DISABLE, &self.buffer, |lck| {
                lck.has_data()
            })?
            .unwrap();

        Ok(buffer.read(buf, len))
    }

    fn write_to_output(&self, symbol: u8, termios: &tty::Termios) {
        if termios.has_lflag(tty::ECHO) {
            self.output.lock_irq().put_char(symbol as u8);
        }
    }

    fn write_to_buffer(&self, symbol: u8, termios: &tty::Termios) {
        let mut buf = self.buffer.lock_irq();
        buf.put_char(symbol as u8);
        if !termios.has_lflag(tty::ICANON) || symbol == b'\n' {
            {
                buf.commit_write();
            }
            if let Some(_t) = &*self.ctrl_task.lock_irq() {
                self.wait_queue.notify_all();
            }
        }
    }

    fn write_symbol(&self, symbol: u8) {
        let termios = self.termios.lock_irq();

        self.write_to_output(symbol, &termios);
        self.write_to_buffer(symbol, &termios);
    }

    fn handle_key_state(key: KeyCode, released: bool, state: &mut SpinGuard<State>) -> bool {
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
            KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => {
                return true;
            }
            _ => {
                return false;
            }
        }

        return true;
    }

    fn handle_canonical(&self, key: KeyCode, released: bool, state: &mut SpinGuard<State>) -> bool {
        match key {
            KeyCode::KEY_BACKSPACE if !released => {
                let n = self.buffer.lock_irq().remove_last_n(1);
                if n > 0 {
                    self.output.lock_irq().remove_last_n(n);
                }
            }
            KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER if !released => {
                self.write_symbol(b'\n' as u8);
            }
            KeyCode::KEY_U if (state.lctrl || state.rctrl) && !released => {
                let n = self.buffer.lock_irq().remove_all_edit();
                if n > 0 {
                    self.output.lock_irq().remove_last_n(n);
                }
            }
            KeyCode::KEY_PAGEDOWN if !released => {
                self.output.lock_irq().scroll_down(20);
            }
            KeyCode::KEY_PAGEUP if !released => {
                self.output.lock_irq().scroll_up(20);
            }
            KeyCode::KEY_HOME if !released => {
                self.output.lock_irq().scroll_top();
            }
            KeyCode::KEY_END if !released => {
                self.output.lock_irq().scroll_bottom();
            }
            KeyCode::KEY_UP if !released => {
                self.output.lock_irq().scroll_up(1);
            }
            KeyCode::KEY_DOWN if !released => {
                self.output.lock_irq().scroll_down(1);
            }
            _ => {
                return false;
            }
        };

        return true;
    }
    fn handle_sig(&self, key: KeyCode, released: bool, state: &mut SpinGuard<State>) -> bool {
        match key {
            KeyCode::KEY_C if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.fg_group.lock_irq().as_ref() {
                    t.signal(SIGINT);
                }
            }
            KeyCode::KEY_Z if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.fg_group.lock_irq().clone() {
                    t.signal(SIGTSTP);
                }
            }
            KeyCode::KEY_D if (state.lctrl || state.rctrl) && !released => {
                self.buffer.lock_irq().trigger_eof();

                self.wait_queue.notify_one();
            }
            KeyCode::KEY_BACKSLASH if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.fg_group.lock_irq().as_ref() {
                    t.signal(SIGQUIT);
                }
            }
            _ => {
                return false;
            }
        };

        return true;
    }

    fn output_sym(&self, key: KeyCode, state: SpinGuard<State>, canonical: bool) {
        if !canonical {
            if let Some(seq) = state.map_raw_sequence(key) {
                let mut buf = self.buffer.lock_irq();
                for v in seq {
                    buf.put_char(*v);
                }
                buf.commit_write();
                if let Some(_t) = &*self.ctrl_task.lock_irq() {
                    self.wait_queue.notify_all();
                }
                return;
            }
        }

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

            if !canonical && state.alt {
                self.write_symbol(0x1b);
            }

            self.write_symbol(sym as u8);
        }
    }
}

impl KeyListener for Tty {
    fn on_new_key(&self, key: KeyCode, released: bool) {
        let canon = self.termios.lock_irq().has_lflag(tty::ICANON);
        let sig = self.termios.lock_irq().has_lflag(tty::ISIG);
        let mut state = self.state.lock();

        if Self::handle_key_state(key, released, &mut state) {
            return;
        }

        if canon && self.handle_canonical(key, released, &mut state) {
            return;
        }

        if sig && self.handle_sig(key, released, &mut state) {
            return;
        }

        if !released {
            self.output_sym(key, state, canon);
        }
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
        logln2!("try tty read");
        let r = self.read(buf.as_mut_ptr(), buf.len());
        logln2!("tty read {:?} {:?}", r, buf);

        match r {
            Ok(s) => Ok(s),
            Err(e) => {
                logln2!("tty signal error");
                Err(e.into())
            }
        }
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize, FsError> {
        if let Err(_) = self.write_str(unsafe { core::str::from_utf8_unchecked(buf) }) {
            Err(FsError::InvalidParam)
        } else {
            Ok(buf.len())
        }
    }

    fn poll(
        &self,
        ptable: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> Result<PollEventFlags, FsError> {
        let mut res_flags = PollEventFlags::empty();
        if flags.contains(PollEventFlags::WRITE) {
            res_flags.insert(PollEventFlags::WRITE);
        }

        if !flags.contains(PollEventFlags::READ) {
            return Ok(res_flags);
        }

        let has_data = self.buffer.lock_irq().has_data();

        if let Some(p) = ptable {
            p.listen(&self.wait_queue);
        }

        if has_data {
            res_flags.insert(PollEventFlags::READ);
        }

        Ok(res_flags)
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
        logln2!("TTY ioctl 0x{:x}", cmd);
        match cmd {
            tty::TIOCSCTTY => {
                let current = current_task_ref();

                if !current.is_session_leader() {
                    return Err(FsError::NoPermission);
                }

                if current_task_ref().terminal().connect(tty().clone()) {
                    logln!("TERMINAL ATTACHED TO TASK {}", current.tid());
                    Ok(0)
                } else {
                    Err(FsError::EntryExists)
                }
            }
            tty::TIOCNOTTY => {
                if current_task_ref()
                    .terminal()
                    .disconnect(Some(tty().clone()))
                {
                    Ok(0)
                } else {
                    Err(FsError::EntryNotFound)
                }
            }
            tty::TIOCGPGRP => {
                let task = current_task_ref();

                if !task.terminal().is_connected(tty().clone()) {
                    logln2!("get is not connected");
                    return Err(FsError::NoPermission);
                }

                let gid = unsafe { VirtAddr(arg).read_mut::<u32>() };

                if let Some(fg) = &*tty().fg_group.lock() {
                    *gid = fg.id() as u32;
                }

                return Ok(0);
            }
            tty::TIOCSPGRP => {
                let gid = unsafe { VirtAddr(arg).read::<u32>() };

                let task = current_task_ref();

                if !task.terminal().is_connected(tty().clone()) {
                    logln2!("set is not connected");
                    return Err(FsError::NoPermission);
                }

                if let Some(ctrl) = self.ctrl_task.lock_irq().as_ref() {
                    if ctrl.sid() == task.sid() {
                        if let Some(group) = sessions().get_group(task.sid(), gid as usize) {
                            self.set_fg_group(group);

                            return Ok(0);
                        } else {
                            logln2!("group {} not found", gid);
                        }
                    } else {
                        logln2!("diff sid {} {}", ctrl.sid(), task.sid());
                    }
                }

                logln2!("set is not auth");
                Err(FsError::NoPermission)
            }
            tty::TIOCSWINSZ => Err(FsError::NoTty),
            tty::TIOCGWINSZ => {
                let winsize =
                    unsafe { VirtAddr(arg).read_mut::<syscall_defs::ioctl::tty::WinSize>() };

                let (cols, rows) = video().dimensions();

                winsize.ws_col = cols as u16;
                winsize.ws_row = rows as u16;

                Ok(0)
            }
            ioctl @ (tty::TCSETS | tty::TCSETSW | tty::TCSETSF) => {
                let termios =
                    unsafe { VirtAddr(arg).read_ref::<syscall_defs::ioctl::tty::Termios>() };

                if let Some(fg) = &*self.fg_group.lock_irq() {
                    let task = current_task_ref();

                    if !fg.has_process(task.pid()) {
                        task.signal(syscall_defs::signal::SIGTTOU);
                        return Err(FsError::Interrupted);
                    }
                }

                logln3!("termios TCSETS 0x{:x}", termios.c_lflag);
                logln3!("{:?}", termios);

                *self.termios.lock_irq() = *termios;

                if ioctl == tty::TCSETSF {
                    self.buffer.lock_irq().flush();
                }

                Ok(0)
            }
            tty::TCGETS => {
                let termios =
                    unsafe { VirtAddr(arg).read_mut::<syscall_defs::ioctl::tty::Termios>() };

                logln!("termios TCGETS 0x{:x}", termios.c_lflag);
                logln!("{:?}", termios);

                *termios = *self.termios.lock_irq();

                Ok(0)
            }
            _ => Err(FsError::NoTty),
        }
    }
}

impl ConsoleWriter for Tty {
    fn write_str(&self, s: &str) -> core::fmt::Result {
        self.output.lock_irq().write_str(s);
        Ok(())
    }
}

lazy_static! {
    static ref TTY: Arc<Tty> = Tty::new();
}

fn tty() -> &'static Arc<Tty> {
    &TTY
}

pub fn init() {
    crate::kernel::kbd::register_key_listener(tty().as_ref());
    if let Err(v) = crate::kernel::device::register_device(tty().clone()) {
        panic!("Failed to register Tty device: {:?}", v);
    }
    if let Err(v) = crate::kernel::tty::register_tty(tty().clone()) {
        panic!("Failed to register Tty terminal {:?}", v);
    }
    crate::arch::output::register_output_driver(tty().as_ref());
    video().clear();
    video().set_cursor_visible(true);

    video().init_dev();
}
