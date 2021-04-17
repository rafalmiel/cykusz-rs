use crate::drivers::ps2::{controller, PS2Controller};
use crate::kernel::sync::Spin;

use super::scancode;

struct KbdState {
    state: Spin<State>,
}

struct State {
    e: bool,
    f: bool,
}

impl State {
    const fn new() -> State {
        State { e: false, f: false }
    }
}

static KEYBOARD: KbdState = KbdState {
    state: Spin::new(State::new()),
};

impl KbdState {
    fn handle_interrupt(&self) {
        let data = controller().read();

        let mut state = self.state.lock();

        match data {
            0xf0 => {
                state.f = true;
                return;
            }
            0xe0 => {
                state.e = true;
                return;
            }
            _ => {
                let released = state.f;

                let key = scancode::get(data as usize, state.e);

                state.e = false;
                state.f = false;

                drop(state);

                crate::kernel::kbd::key_notify(key, released);
            }
        }
    }
}

pub fn handle_interrupt() {
    KEYBOARD.handle_interrupt();
}
