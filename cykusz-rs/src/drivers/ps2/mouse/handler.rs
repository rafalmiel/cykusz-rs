use alloc::string::String;
use alloc::sync::{Arc, Weak};

use bit_field::BitField;
use spin::Once;

use syscall_defs::events::buttons::{ButtonCode, RelCode};
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
use crate::kernel::utils::spsc_queue::SPSCQueue;
use crate::kernel::utils::wait_queue::{WaitQueue, WaitQueueFlags};

struct MouseState {
    state: Spin<State>,
    dev_id: DevId,
    self_ref: Weak<MouseState>,

    spsc: SPSCQueue<'static, Event, 128>,
    wq: WaitQueue,
}

struct State {
    packet: [u8; 4],
    index: usize,
    opened: bool,
    btn_state: [bool; 3],
}

impl State {
    fn new() -> State {
        State {
            packet: [0; 4],
            index: 0,
            opened: false,
            btn_state: [false; 3], //btn left, btn right, btn middle, btn side, btn extra
        }
    }

    fn iter(&mut self) -> StateIter<'_> {
        StateIter {
            state: self,
            idx: 0,
        }
    }
}

struct StateIter<'a> {
    state: &'a mut State,
    idx: usize,
}

impl<'a> StateIter<'a> {
    fn btn_code(&self) -> Option<ButtonCode> {
        match self.idx {
            0 => Some(ButtonCode::BTN_LEFT),
            1 => Some(ButtonCode::BTN_RIGHT),
            2 => Some(ButtonCode::BTN_MIDDLE),
            _ => None,
        }
    }

    fn get_rel_x(&self) -> i32 {
        let d = self.state.packet[1] as i32;
        d - (((self.state.packet[0] as i32) << 4) & 0x100)
    }

    fn get_rel_y(&self) -> i32 {
        let d = self.state.packet[2] as i32;
        d - (((self.state.packet[0] as i32) << 3) & 0x100)
    }
}

impl<'a> Iterator for StateIter<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(btn_code) = self.btn_code() {
            let idx = self.idx;
            self.idx += 1;
            if self.state.btn_state[idx] != self.state.packet[0].get_bit(idx) {
                self.state.btn_state[idx] = self.state.packet[0].get_bit(idx);

                return Some(Event {
                    timeval: Timeval::from_nsecs(current_ns()),
                    typ: EventType::Key,
                    code: btn_code as u16,
                    val: if self.state.btn_state[idx] { 1 } else { 0 },
                });
            }
        }

        if self.idx == 3 {
            self.idx += 1;
            let rel_x = self.get_rel_x();
            if rel_x != 0 {
                return Some(Event {
                    timeval: Timeval::from_nsecs(current_ns()),
                    typ: EventType::Rel,
                    code: RelCode::REL_X as u16,
                    val: rel_x,
                });
            }
        }

        if self.idx == 4 {
            self.idx += 1;
            let rel_y = self.get_rel_y();
            if rel_y != 0 {
                return Some(Event {
                    timeval: Timeval::from_nsecs(current_ns()),
                    typ: EventType::Rel,
                    code: RelCode::REL_Y as u16,
                    val: rel_y,
                });
            }
        }

        None
    }
}

impl Device for MouseState {
    fn id(&self) -> DevId {
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
    fn read_at(
        &self,
        _offset: usize,
        buf: &mut [u8],
        flags: OpenFlags,
    ) -> crate::kernel::fs::vfs::Result<usize> {
        dbgln!(mouse, "read_at");
        if buf.len() % core::mem::size_of::<Event>() != 0 {
            dbgln!(mouse, "Failed mouse read of {} bytes", buf.len());
            Err(FsError::InvalidParam)
        } else {
            if let Some(()) = self.wq.wait_for(
                WaitQueueFlags::from(flags) | WaitQueueFlags::IRQ_DISABLE,
                || self.spsc.has_data(),
            )? {
                let res = self.spsc.try_read_one();
                dbgln!(mouse, "trying to read one {:?}", res);
                if let Some(e) = res {
                    dbgln!(mouse, "got {:?}", e);
                    buf.copy_from_slice(unsafe {
                        core::slice::from_raw_parts(&e as *const _ as *const u8, size_of::<Event>())
                    });

                    Ok(size_of::<Event>())
                } else {
                    dbgln!(mouse, "read failed");
                    Ok(0)
                }
            } else {
                Ok(0)
            }
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
            pt.listen(&self.wq);
        }

        Ok(if self.spsc.has_data() {
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

        if state.index == 0 && state.opened {
            if let Some(evt) = state.iter().next() {
                drop(state);

                let res = self.spsc.try_write_one(&evt);
                dbgln!(mouse, "int write evt: {:?} {:?}", evt, res);
                self.wq.notify_one();
            }
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
            dev_id: crate::kernel::device::alloc_id(),
            self_ref: me.clone(),

            spsc: SPSCQueue::new(),
            wq: WaitQueue::new(),
        })
    });

    mouse().spsc.init();

    crate::kernel::device::register_device(mouse().clone())
        .expect("Failed to register keyboard device")
}

pub fn handle_interrupt() {
    if let Err(e) = mouse().handle_interrupt() {
        logln6!("mouse interrupt error: {:?}", e);
    }
}
