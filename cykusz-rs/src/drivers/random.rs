use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;
use alloc::string::String;
use alloc::sync::{Arc, Weak};

use crate::kernel::device::dev_t::DevId;
use crate::kernel::sync::Spin;
use crate::kernel::timer::current_ns;
use rand::{RngCore, SeedableRng};

struct Random {
    id: DevId,
    name: String,
    sref: Weak<Random>,
    rng: Spin<rand::prelude::StdRng>,
}

impl Random {
    fn new(name: String) -> Arc<Random> {
        let mut seed: [u8; 32] = [0; 32];

        for s in &mut seed {
            *s = current_ns() as u8;
        }

        Arc::new_cyclic(|me| Random {
            id: crate::kernel::device::alloc_id(),
            name,
            sref: me.clone(),
            rng: Spin::new(rand::prelude::StdRng::from_seed(seed)),
        })
    }

    fn me(&self) -> Arc<Random> {
        self.sref.upgrade().unwrap()
    }
}

impl INode for Random {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> crate::kernel::fs::vfs::Result<usize> {
        self.rng.lock().fill_bytes(buf);

        Ok(buf.len())
    }
}

impl Device for Random {
    fn id(&self) -> DevId {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.me()
    }
}

pub fn init() {
    let _ = crate::kernel::device::register_device(Random::new("random".into()));
    let _ = crate::kernel::device::register_device(Random::new("urandom".into()));
}

module_init!(init);
