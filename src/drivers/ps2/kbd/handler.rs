use crate::drivers::ps2::{PS2Controller, controller};
use super::scancode;
use crate::kernel::sync::Mutex;
use crate::kernel::utils::wait_queue::WaitQueue;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use crate::drivers::input::KeyListener;
use alloc::vec::Vec;

struct KbdState {
    buffer: Mutex<Buffer>,
    wait_queue: WaitQueue,
    state: Mutex<State>,
}

struct Buffer {
    buffer: [u8; 64],
    data_begin: usize,
    data_end: usize,
}

struct State {
    e: bool,
    f: bool,
}

impl State {
    const fn new() -> State {
        State {
            e: false,
            f: false,
        }
    }
}

impl Buffer {
    const fn new() -> Buffer {
        Buffer {
            buffer: [0; 64],
            data_begin: 0,
            data_end: 0,
        }
    }

    fn append_data(&mut self, data: u8) {
        let pos = (self.data_begin + 1) % 64;

        self.data_end += 1;

        self.buffer[pos] = data;
    }

    fn has_data(&self) -> bool {
        self.data_begin != self.data_end
    }

    fn read(&mut self) {
        self.data_begin = self.data_end;
    }
}

static KEYBOARD: KbdState = KbdState {
    buffer: Mutex::new(Buffer::new()),
    wait_queue: WaitQueue::new(),
    state: Mutex::new(State::new()),
};

impl KbdState {

    fn read(&self) {

        while !self.buffer.lock_irq().has_data() {
            use crate::kernel::sched::current_task;

            self.wait_queue.add_task(current_task().clone());
        }

        self.buffer.lock().read();
    }

    fn handle_interrupt(&self) {
        let data = controller().read();

        let mut state = self.state.lock();

        match data {
            0xf0 => {
                state.f = true;
                return;
            },
            0xe0 => {
                state.e = true;
                return;
            },
            _ => {
                let released = state.f;

                let key = scancode::get(data as usize, state.e);

                state.e = false;
                state.f = false;

                drop(state);

                println!("0x{:x}", data);

                crate::drivers::input::key_notify(key, released);
            }
        }
   }
}

pub fn read() {
    KEYBOARD.read();
}

pub fn handle_interrupt() {
    KEYBOARD.handle_interrupt();
}
