use crate::drivers::ps2::{controller, Error};
use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::sync::Spin;
use crate::kernel::utils::buffer::BufferQueue;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use spin::Once;
use syscall_defs::poll::PollEventFlags;
use syscall_defs::OpenFlags;

struct MouseState {
    state: Spin<State>,
    dev_id: usize,
    buf: BufferQueue,
    self_ref: Weak<MouseState>,
}

struct State {
    packet: [u8; 4],
    index: usize,
    opened: bool,
}

impl State {
    fn new() -> State {
        State {
            packet: [0; 4],
            index: 0,
            opened: false,
        }
    }
}

impl Device for MouseState {
    fn id(&self) -> usize {
        self.dev_id
    }

    fn name(&self) -> String {
        String::from("mouse")
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.self_ref.upgrade().unwrap()
    }
}

impl INode for MouseState {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> crate::kernel::fs::vfs::Result<usize> {
        if buf.len() % 4 != 0 {
            Err(FsError::InvalidParam)
        } else {
            Ok(self.buf.read_data(buf)?)
        }
    }

    fn poll(
        &self,
        poll_table: Option<&mut PollTable>,
        flags: PollEventFlags,
    ) -> crate::kernel::fs::vfs::Result<PollEventFlags> {
        if !flags.contains(PollEventFlags::READ) {
            return Err(FsError::NotSupported);
        }

        if let Some(pt) = poll_table {
            pt.listen(&self.buf.readers_queue());
        }

        Ok(if self.buf.has_data() {
            PollEventFlags::READ
        } else {
            PollEventFlags::empty()
        })
    }

    fn open(&self, _flags: OpenFlags) -> crate::kernel::fs::vfs::Result<()> {
        self.state.lock_irq().opened = true;

        Ok(())
    }

    fn close(&self, _flags: OpenFlags) {
        self.state.lock_irq().opened = false;
    }
}

impl MouseState {
    fn handle_interrupt(&self) -> Result<(), Error> {
        let data = controller().read()?;

        let mut state = self.state.lock_irq();

        let idx = state.index;

        state.packet[idx] = data;
        state.index = (state.index + 1) % 4;

        if state.opened {
            self.buf.try_append_data(&state.packet);
        }

        Ok(())
    }
}

static MOUSE: Once<Arc<MouseState>> = Once::new();

fn mouse() -> &'static Arc<MouseState> {
    unsafe { MOUSE.get_unchecked() }
}

pub fn init() {
    MOUSE.call_once(|| {
        Arc::new_cyclic(|me| MouseState {
            state: Spin::new(State::new()),
            buf: BufferQueue::new(4 * 32),
            dev_id: crate::kernel::device::alloc_id(),
            self_ref: me.clone(),
        })
    });

    crate::kernel::device::register_device(mouse().clone())
        .expect("Failed to register keyboard device")
}

pub fn handle_interrupt() {
    if let Err(e) = mouse().handle_interrupt() {
        logln6!("mouse interrupt error: {:?}", e);
    }
}
