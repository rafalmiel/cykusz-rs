use alloc::string::String;
use alloc::sync::{Arc, Weak};
use core::fmt::Debug;
use core::fmt::{Error, Formatter};

use crate::arch::output::ConsoleDriver;
use crate::drivers::input::keymap;
use crate::drivers::input::keys::KeyCode;
use crate::drivers::input::KeyListener;
use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::sched::current_task;
use crate::kernel::signal::SignalResult;
use crate::kernel::sync::Spin;
use crate::kernel::syscall::sys::PollTable;
use crate::kernel::task::Task;
use crate::kernel::tty::TerminalDevice;
use crate::kernel::utils::wait_queue::WaitQueue;

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

const BUFFER_SIZE: usize = 256;

struct Buffer {
    data: [u8; BUFFER_SIZE],
    e: u32,
    w: u32,
    r: u32,
}

impl Buffer {
    const fn new() -> Buffer {
        Buffer {
            data: [0u8; BUFFER_SIZE],
            e: 0,
            w: 0,
            r: 0,
        }
    }

    fn put_char(&mut self, data: u8) {
        if (self.e + 1) % BUFFER_SIZE as u32 != self.r {
            self.data[self.e as usize] = data;
            self.e = (self.e + 1) % BUFFER_SIZE as u32;
            print!("{}", data as char);
        }
    }

    fn remove_all_edit(&mut self) -> usize {
        let edit_size = if self.e < self.w {
            BUFFER_SIZE as u32 - (self.w - self.e)
        } else {
            self.e - self.w
        };

        self.e = self.w;

        edit_size as usize
    }

    fn remove_last_n(&mut self, n: usize) -> usize {
        let mut remaining = n;

        while self.e != self.w && remaining > 0 {
            self.e = if self.e == 0 {
                BUFFER_SIZE as u32 - 1
            } else {
                self.e - 1
            };

            remaining -= 1;
        }

        n - remaining
    }

    fn read(&mut self, buf: *mut u8, n: usize) -> usize {
        let mut remaining = n;
        let mut store = buf;

        while self.r != self.w && remaining > 0 {
            unsafe {
                *store = self.data[self.r as usize];

                store = store.offset(1);
            }

            remaining -= 1;
            self.r = (self.r + 1) % BUFFER_SIZE as u32;
        }

        n - remaining
    }

    fn commit_write(&mut self) {
        self.w = self.e;
    }

    fn has_data(&self) -> bool {
        self.r != self.w
    }
}

struct Tty {
    dev_id: usize,
    state: Spin<State>,
    buffer: Spin<Buffer>,
    wait_queue: WaitQueue,
    ctrl_task: Spin<Option<Arc<Task>>>,
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
    fn new() -> Tty {
        Tty {
            dev_id: crate::kernel::device::alloc_id(),
            state: Spin::new(State::new()),
            buffer: Spin::new(Buffer::new()),
            wait_queue: WaitQueue::new(),
            ctrl_task: Spin::new(None),
            self_ptr: Weak::default(),
        }
    }

    fn wrap(self) -> Arc<Self> {
        let arc = Arc::new(self);

        let weak = Arc::downgrade(&arc);
        let ptr = Arc::into_raw(arc) as *mut Self;

        unsafe {
            (*ptr).self_ptr = weak;
            Arc::from_raw(ptr)
        }
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
                use crate::arch::output::writer;
                let n = self.buffer.lock().remove_last_n(1);
                if n > 0 {
                    let w = writer();
                    w.remove_last_n(n);
                }
            }
            KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER if !released => {
                {
                    let mut buf = self.buffer.lock();
                    buf.put_char('\n' as u8);
                    buf.commit_write();
                }
                if let Some(t) = &*self.ctrl_task.lock() {
                    self.wait_queue.notify_all();
                }
            }
            KeyCode::KEY_U if (state.lctrl || state.rctrl) && !released => {
                use crate::arch::output::writer;
                let n = self.buffer.lock().remove_all_edit();
                if n > 0 {
                    let w = writer();
                    w.remove_last_n(n);
                }
            }
            KeyCode::KEY_C if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.ctrl_task.lock().clone() {
                    t.signal(syscall_defs::signal::SIGINT);
                }
            }
            KeyCode::KEY_BACKSLASH if (state.lctrl || state.rctrl) && !released => {
                if let Some(t) = self.ctrl_task.lock().clone() {
                    t.signal(syscall_defs::signal::SIGQUIT);
                }
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

    fn attach(&self, task: Arc<Task>) -> bool {
        let mut cur = self.ctrl_task.lock();

        if cur.is_none() {
            println!("[ TTY ] Attached task {}", task.id());
            *cur = Some(task);

            return true;
        }

        false
    }

    fn detach(&self, task: Arc<Task>) -> bool {
        let mut ctrl = self.ctrl_task.lock();

        if let Some(cur) = ctrl.as_ref() {
            if cur.id() == task.id() {
                println!("[ TTY ] Detached task {}", cur.id());
                *ctrl = None;

                return true;
            }
        }

        false
    }
}

impl INode for Tty {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize, FsError> {
        Ok(self.read(buf.as_mut_ptr(), buf.len())?)
    }

    fn poll(&self, ptable: Option<&mut PollTable>) -> Result<bool, FsError> {
        let has_data = self.buffer.lock().has_data();

        if let Some(p) = ptable {
            p.listen(&self.wait_queue);
        }

        Ok(has_data)
    }

    fn ioctl(&self, cmd: usize, _arg: usize) -> Result<usize, FsError> {
        match cmd {
            syscall_defs::ioctl::tty::TIOCSCTTY => {
                if current_task().terminal().attach(tty().clone()) {
                    Ok(0)
                } else {
                    Err(FsError::EntryExists)
                }
            }
            syscall_defs::ioctl::tty::TIOCNOTTY => {
                if current_task().terminal().detach_term(tty().clone()) {
                    Ok(0)
                } else {
                    Err(FsError::EntryNotFound)
                }
            }
            _ => Err(FsError::NotSupported),
        }
    }
}

lazy_static! {
    static ref TTY: Arc<Tty> = Tty::new().wrap();
}

fn tty() -> &'static Arc<Tty> {
    &TTY
}

pub fn read(buf: *mut u8, len: usize) -> SignalResult<usize> {
    let l = tty();

    l.read(buf, len)
}

pub fn poll_listen(ptable: Option<&mut PollTable>) -> Result<bool, FsError> {
    tty().poll(ptable)
}

fn init() {
    crate::drivers::input::register_key_listener(tty().as_ref());
    if let Err(v) = crate::kernel::device::register_device(tty().clone()) {
        panic!("Failed to register Tty device: {:?}", v);
    }
    if let Err(v) = crate::kernel::tty::register_tty(tty().clone()) {
        panic!("Failed to register Tty terminal {:?}", v);
    }
}

module_init!(init);
