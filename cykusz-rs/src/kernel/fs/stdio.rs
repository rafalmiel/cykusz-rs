use alloc::string::String;
use alloc::sync::Arc;
use spin::Once;

use crate::kernel::device::{alloc_id, Device, register_device};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::Result;
use crate::kernel::syscall::sys::PollTable;

pub struct StdOut {
    id: usize,
}

impl INode for StdOut {
    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        print!("{}", unsafe { core::str::from_utf8_unchecked(buf) });
        Ok(buf.len())
    }
}

pub struct StdIn {
    id: usize,
}

impl INode for StdIn {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(crate::drivers::tty::read(buf.as_mut_ptr(), buf.len())?)
    }

    fn poll(&self, ptable: Option<&mut PollTable>) -> Result<bool> {
        crate::drivers::tty::poll_listen(ptable)
    }
}

static STDOUT: Once<Arc<StdOut>> = Once::new();
static STDIN: Once<Arc<StdIn>> = Once::new();

impl Device for StdIn {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> String {
        String::from("stdin")
    }

    fn inode(&self) -> Arc<dyn INode> {
        stdin().clone()
    }
}

impl Device for StdOut {
    fn id(&self) -> usize {
        self.id
    }

    fn name(&self) -> String {
        String::from("stdout")
    }

    fn inode(&self) -> Arc<dyn INode> {
        stdout().clone()
    }
}

pub fn stdout() -> &'static Arc<StdOut> {
    &STDOUT.get().unwrap()
}

pub fn stdin() -> &'static Arc<StdIn> {
    &STDIN.get().unwrap()
}

pub fn init() {
    STDOUT.call_once(|| Arc::new(StdOut { id: alloc_id() }));
    STDIN.call_once(|| Arc::new(StdIn { id: alloc_id() }));

    register_device(stdin().clone()).expect("Failed to register stdin device");
    register_device(stdout().clone()).expect("Failed to register stdout device");
}
