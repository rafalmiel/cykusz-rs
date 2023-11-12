use alloc::string::String;
use alloc::sync::{Arc, Weak};
use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;

use rand;
use rand::{RngCore, SeedableRng};

struct Random {
    id: usize,
    name: String,
    sref: Weak<Random>,
}

impl Random {
    fn new(name: String) -> Arc<Random> {
        Arc::new_cyclic(|me| {
            Random {
                id: crate::kernel::device::alloc_id(),
                name,
                sref: me.clone()
            }
        })
    }

    fn me(&self) -> Arc<Random> {
        self.sref.upgrade().unwrap()
    }
}

impl INode for Random {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> crate::kernel::fs::vfs::Result<usize> {
        let mut rnd = rand::prelude::StdRng::from_seed(Default::default());

        rnd.fill_bytes(buf);

        Ok(buf.len())
    }
}

impl Device for Random {
    fn id(&self) -> usize {
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