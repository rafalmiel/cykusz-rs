use alloc::string::String;
use alloc::sync::{Arc, Weak};
use syscall_defs::OpenFlags;
use crate::kernel::device::dev_t::DevId;
use crate::kernel::device::{register_device, Device};
use crate::kernel::fs::inode::INode;

struct NullDev {
    id: DevId,
    name: String,
    sref: Weak<NullDev>,
}

impl NullDev {
    fn new() -> Arc<NullDev> {
        Arc::new_cyclic(|me| {
            NullDev {
                id: crate::kernel::device::alloc_id(),
                name: "null".into(),
                sref: me.clone()
            }
        })
    }
}

impl INode for NullDev {
    fn read_at(&self, _offset: usize, _buf: &mut [u8], _flags: OpenFlags) -> crate::kernel::fs::vfs::Result<usize> {
        Ok(0)
    }

    fn write_at(&self, _offset: usize, buf: &[u8], _flags: OpenFlags) -> crate::kernel::fs::vfs::Result<usize> {
        Ok(buf.len())
    }
}

impl Device for NullDev {
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
    register_device(NullDev::new()).expect("Failed to register null device");
}

module_init!(init);
