use alloc::vec::Vec;

use syscall_defs::events::keys::KeyCode;

use crate::kernel::sync::RwSpin;

pub trait KeyListener: Sync {
    fn on_new_key(&self, key: KeyCode, released: bool);
}

static LISTENERS: RwSpin<Vec<&'static dyn KeyListener>> = RwSpin::new(Vec::new());

pub fn key_notify(key: KeyCode, released: bool) {
    for l in LISTENERS.read().iter() {
        l.on_new_key(key, released);
    }
}

pub fn register_key_listener(listener: &'static dyn KeyListener) {
    LISTENERS.write().push(listener);
}
