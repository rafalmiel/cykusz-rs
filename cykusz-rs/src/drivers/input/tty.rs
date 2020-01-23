use crate::drivers::input::KeyListener;
use crate::drivers::input::keymap;
use crate::drivers::input::keys::KeyCode;
use crate::kernel::sync::Mutex;
use core::fmt::Debug;
use core::fmt::{Formatter, Error};
use crate::kernel::utils::wait_queue::WaitQueue;
use crate::arch::output::ConsoleDriver;

struct State {
    lshift: bool,
    rshift: bool,
    caps: bool,
    lctrl: bool,
    rctrl: bool,
    alt: bool,
    altgr: bool
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "({:<8} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8})",
               self.lshift, self.rshift, self.caps, self.lctrl, self.rctrl, self.alt, self.altgr)
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

    fn remove_last_n(&mut self, n: usize) -> usize {
        let mut remaining = n;

        while self.e != self.w && remaining > 0 {
            if self.e == 0 {
                self.e = BUFFER_SIZE as u32 - 1;
            } else {
                self.e -= 1;
            }

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
    state: Mutex<State>,
    buffer: Mutex<Buffer>,
    wait_queue: WaitQueue,
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
            altgr: false
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

        match (shift, ctrl, alt, altgr)  {
            (false, false, false, false) => {
                Some(&keymap::PLAIN_MAP)
            },
            (true, false, false, false) => {
                Some(&keymap::SHIFT_MAP)
            },
            (false, true, false, false) => {
                Some(&keymap::CTRL_MAP)
            },
            (false, false, true, false) => {
                Some(&keymap::ALT_MAP)
            },
            (false, false, false, true) => {
                Some(&keymap::ALTGR_MAP)
            },
            (true, true, false, false) => {
                Some(&keymap::CTRL_SHIFT_MAP)
            },
            (false, true, true, false) => {
                Some(&keymap::CTRL_ALT_MAP)
            }
            _ => {
                None
            }
        }
    }
}

impl Tty {
    const fn new() -> Tty {
        Tty {
            state: Mutex::new(State::new()),
            buffer: Mutex::new(Buffer::new()),
            wait_queue: WaitQueue::new(),
        }
    }

    fn read(&self, buf: *mut u8, len: usize) -> usize {
        while !self.buffer.lock().has_data() {
            use crate::kernel::sched::current_task;

            self.wait_queue.add_task(current_task().clone());
        }
        self.buffer.lock().read(buf, len)
    }
}

impl KeyListener for Tty {
    fn on_new_key(&self, key: KeyCode, released: bool) {
        let mut state = self.state.lock();

        match key {
            KeyCode::KEY_CAPSLOCK if !released => {
                state.caps = !state.caps;
            },
            KeyCode::KEY_LEFTSHIFT => {
                state.lshift = !released;
            },
            KeyCode::KEY_RIGHTSHIFT => {
                state.rshift = !released;
            },
            KeyCode::KEY_LEFTCTRL => {
                state.lctrl = !released;
            },
            KeyCode::KEY_RIGHTCTRL => {
                state.rctrl = !released;
            },
            KeyCode::KEY_LEFTALT => {
                state.alt = !released;
            },
            KeyCode::KEY_RIGHTALT => {
                state.altgr = !released;
            },
            KeyCode::KEY_BACKSPACE if !released => {
                use crate::arch::output::writer;
                let n = self.buffer.lock().remove_last_n(1);
                if n > 0 {
                    let w = writer();
                    w.remove_last_n(n);
                    return;
                }
            },
            KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER if !released => {
                self.buffer.lock().commit_write();
                print!("\n");
                self.wait_queue.notify_one();
            },
            _ if !released => {
                if let Some(finalmap) =
                    if let Some(map) = state.map(false) {
                        if !state.caps {
                            Some(map)
                        } else {
                            let val = map[key as usize];

                            if (val >> 8) & 0xff == 0xfb { // 0xfb marker denotes letter than is
                                // affected by caps lock
                                state.map(true)
                            } else {
                                Some(map)
                            }
                        }
                    } else {
                        None
                    } {
                        let sym = ((finalmap[key as usize] & 0xff) as u8) as char;

                        self.buffer.lock().put_char(sym as u8);
                    }
            },
            _ => {}
        };
    }
}

static LISTENER : Tty = Tty::new();

pub fn read(buf: *mut u8, len: usize) -> usize {
    let l = &LISTENER;

    l.read(buf, len)
}

fn init() {
    crate::drivers::input::register_key_listener(&LISTENER);
}

module_init!(init);