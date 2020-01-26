use alloc::string::String;
use alloc::sync::Arc;

use crate::kernel::fs::inode::INode;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::fs::vfs::Result;
use spin::Once;

pub struct StdOut {}

impl INode for StdOut {
    fn id(&self) -> usize {
        0
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        print!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }
}

pub struct StdIn {}

impl INode for StdIn {
    fn id(&self) -> usize {
        0
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Ok(crate::drivers::input::tty::read(buf.as_mut_ptr(), buf.len()))
    }
}

static STDOUT: Once<Arc<StdOut>> = Once::new();
static STDIN: Once<Arc<StdIn>> = Once::new();

pub fn stdout() -> &'static Arc<StdOut> {
    &STDOUT.r#try().unwrap()
}

pub fn stdin() -> &'static Arc<StdIn> {
    &STDIN.r#try().unwrap()
}

pub fn init() {
    STDOUT.call_once(|| {
        Arc::new(StdOut{})
    });
    STDIN.call_once(|| {
        Arc::new(StdIn{})
    });
}
