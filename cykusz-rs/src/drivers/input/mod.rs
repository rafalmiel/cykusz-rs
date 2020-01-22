use crate::drivers::input::keys::KeyCode;
use crate::kernel::sync::Mutex;
use alloc::vec::Vec;

pub mod keys;
pub mod tty;
pub mod keymap;

pub trait KeyListener : Sync {
    fn on_new_key(&self, key: keys::KeyCode, released: bool);
}

static LISTENERS: Mutex<Vec<&'static dyn KeyListener>> = Mutex::new(Vec::new());

pub fn key_notify(key: KeyCode, released: bool) {
    for l in LISTENERS.lock().iter() {
        l.on_new_key(key, released);
    }
}

pub fn register_key_listener(listener: &'static dyn KeyListener) {
    LISTENERS.lock().push(listener);
}
