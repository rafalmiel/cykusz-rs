use crate::cykusz::input::read_event;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use syscall_defs::events::buttons::{ButtonCode, RelCode};
use syscall_defs::events::EventType;
use syscall_defs::poll::{PollEventFlags, PollFd};

pub struct MouseInput {
    file: File,

    mouse_has_data: bool,
    mouse_buttons: (bool, bool, bool),
    mouse_relx: i32,
    mouse_rely: i32,
}

impl MouseInput {
    pub fn new() -> Result<MouseInput, ExitCode> {
        let mouse = File::open("/dev/mouse").map_err(|_| ExitCode::FAILURE)?;

        Ok(MouseInput {
            file: mouse,
            mouse_has_data: false,
            mouse_buttons: (false, false, false),
            mouse_relx: 0,
            mouse_rely: 0,
        })
    }

    pub fn poll_fd(&self) -> PollFd {
        PollFd::new(self.file.as_raw_fd(), PollEventFlags::READ)
    }

    pub fn handle(&mut self) {
        let event = read_event(&mut self.file);

        self.mouse_has_data = true;

        match event.typ {
            EventType::Key => {
                let button: ButtonCode = unsafe { std::mem::transmute(event.code) };
                match button {
                    ButtonCode::BTN_LEFT => {
                        self.mouse_buttons.0 = event.val == 1;
                    }
                    ButtonCode::BTN_RIGHT => {
                        self.mouse_buttons.1 = event.val == 1;
                    }
                    ButtonCode::BTN_MIDDLE => {
                        self.mouse_buttons.2 = event.val == 1;
                    }
                    _ => {}
                }
            }
            EventType::Rel => {
                let rel: RelCode = unsafe { std::mem::transmute(event.code) };

                match rel {
                    RelCode::REL_X => {
                        self.mouse_relx += event.val;
                    }
                    RelCode::REL_Y => {
                        self.mouse_rely += event.val;
                    }
                }
            }
        }
    }

    pub fn get_mouse(&mut self) -> Option<((bool, bool, bool), i32, i32)> {
        if !self.mouse_has_data {
            return None;
        }

        self.mouse_has_data = false;

        // 0 as mouse-y, since we move with keyboard
        let ret = Some((self.mouse_buttons, self.mouse_relx, 0));

        self.mouse_relx = 0;
        self.mouse_rely = 0;

        ret
    }
}
