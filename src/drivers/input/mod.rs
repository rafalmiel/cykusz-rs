use crate::drivers::input::keys::KeyCode;
use crate::kernel::sync::Mutex;
use alloc::vec::Vec;

pub mod keys;
pub mod tty;
pub mod keymap;

pub trait KeyListener : Sync {
    fn on_new_key(&self, key: keys::KeyCode, released: bool);
}

struct Listener {}

impl KeyListener for Listener {
    fn on_new_key(&self, key: KeyCode, released: bool) {
        if !released {
            println!("{:?}", key);
        }
    }
}

static LISTENERS: Mutex<Vec<&'static dyn KeyListener>> = Mutex::new(Vec::new());

static KEY_LISTENER: Listener = Listener {};

pub fn key_notify(key: KeyCode, released: bool) {
    for l in LISTENERS.lock().iter() {
        l.on_new_key(key, released);
    }
}

fn init() {
    LISTENERS.lock().push(&KEY_LISTENER);
}

module_init!(init);