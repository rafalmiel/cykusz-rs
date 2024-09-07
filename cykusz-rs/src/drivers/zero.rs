use alloc::string::String;
use alloc::sync::{Arc, Weak};
use syscall_defs::OpenFlags;
use crate::kernel::device::dev_t::DevId;
use crate::kernel::device::{register_device, Device};
use crate::kernel::fs::inode::INode;

struct ZeroDev {
    id: DevId,
    name: String,
    sref: Weak<ZeroDev>,
}

impl ZeroDev {
    fn new() -> Arc<ZeroDev> {
        Arc::new_cyclic(|me| {
            ZeroDev {
                id: crate::kernel::device::alloc_id(),
                name: "zero".into(),
                sref: me.clone()
            }
        })
    }
}

impl INode for ZeroDev {
    fn read_at(&self, _offset: usize, buf: &mut [u8], _flags: OpenFlags) -> crate::kernel::fs::vfs::Result<usize> {
        buf.fill(0);

        Ok(buf.len())
    }

    fn write_at(&self, _offset: usize, buf: &[u8], _flags: OpenFlags) -> crate::kernel::fs::vfs::Result<usize> {
        Ok(buf.len())
    }
}

impl Device for ZeroDev {
    fn id(&self) -> DevId {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.sref.upgrade().clone().unwrap()
    }
}

fn init() {
    register_device(ZeroDev::new()).expect("Failed to register zero device");
}

module_init!(init);
