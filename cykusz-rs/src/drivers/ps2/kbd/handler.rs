use alloc::string::String;
use alloc::sync::{Arc, Weak};

use spin::Once;

use syscall_defs::events::{Event, EventType};
use syscall_defs::poll::PollEventFlags;
use syscall_defs::time::Timeval;
use syscall_defs::OpenFlags;

use crate::drivers::ps2::{controller, Error};
use crate::kernel::device::dev_t::DevId;
use crate::kernel::device::Device;
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::poll::PollTable;
use crate::kernel::fs::vfs::FsError;
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::timer::current_ns;
use crate::kernel::utils::buffer::BufferQueue;
use crate::kernel::utils::wait_queue::WaitQueueFlags;

use super::scancode;

struct KbdState {
    state: Spin<State>,
    dev_id: DevId,
    buf: BufferQueue,
    self_ref: Weak<KbdState>,
}

impl Device for KbdState {
    fn id(&self) -> DevId {
        self.dev_id
    }

    fn name(&self) -> String {
        "kbd".into()
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.self_ref.upgrade().unwrap()
    }
}

impl INode for KbdState {
    fn read_at(
        &self,
        _offset: usize,
        buf: &mut [u8],
        flags: OpenFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        if buf.len() % core::mem::size_of::<Event>() != 0 {
            Err(FsError::InvalidParam)
        } else {
            Ok(self.buf.read_data_flags(
                buf,
                WaitQueueFlags::IRQ_DISABLE | WaitQueueFlags::from(flags),
            )?)
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

    fn open(&self, flags: OpenFlags) -> crate::kernel::fs::vfs::Result<()> {
        if flags.is_open_mode(OpenFlags::RDONLY) {
            self.state.lock_irq().opened = true;
            Ok(())
        } else {
            Err(FsError::NoPermission)
        }
    }

    fn close(&self, _flags: OpenFlags) {
        self.state.lock_irq().opened = false;
    }
}

struct State {
    e: bool,
    f: bool,
    opened: bool,
    pressed: bitmaps::Bitmap<256>,
}

impl State {
    fn new() -> State {
        State {
            e: false,
            f: false,
            opened: false,
            pressed: bitmaps::Bitmap::new(),
        }
    }
}

static KEYBOARD: Once<Arc<KbdState>> = Once::new();

impl KbdState {
    fn handle_interrupt(&self) -> Result<(), Error> {
        let data = controller().read()?;

        let mut state = self.state.lock_irq();

        match data {
            0xf0 => {
                state.f = true;
            }
            0xe0 => {
                state.e = true;
            }
            _ => {
                let released = state.f;

                //logln!("got scancode: {}", data);

                let key = scancode::get(data as usize, state.e);

                state.e = false;
                state.f = false;

                let opened = state.opened;

                let was_pressed = state.pressed.get(key as usize);
                state.pressed.set(key as usize, !released);

                drop(state);

                crate::kernel::kbd::key_notify(key, released);

                if opened {
                    let repeat = !released && was_pressed;
                    let evt = Event {
                        timeval: Timeval::from_nsecs(current_ns()),
                        typ: EventType::Key,
                        code: key as u16,
                        val: if released {
                            1
                        } else if repeat {
                            2
                        } else {
                            0
                        },
                    };

                    unsafe {
                        let bytes: &[u8] = core::slice::from_raw_parts(
                            &evt as *const Event as *const u8,
                            core::mem::size_of::<Event>(),
                        );
                        self.buf.try_append_data_irq(bytes);
                    }
                }
            }
        }
        return Ok(());
    }
}

fn keyboard() -> &'static Arc<KbdState> {
    unsafe { KEYBOARD.get_unchecked() }
}

pub fn init() {
    KEYBOARD.call_once(|| {
        Arc::new_cyclic(|me| KbdState {
            state: Spin::new(State::new()),
            buf: BufferQueue::new(core::mem::size_of::<Event>() * 32, false, true),
            dev_id: crate::kernel::device::alloc_id(),
            self_ref: me.clone(),
        })
    });

    crate::kernel::device::register_device(keyboard().clone())
        .expect("Failed to register keyboard device")
}

pub fn handle_interrupt() {
    if let Err(e) = keyboard().handle_interrupt() {
        logln6!("kbd interrupt error: {:?}", e);
    }
}
