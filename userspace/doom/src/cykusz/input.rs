use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use syscall_defs::events::buttons::{ButtonCode, RelCode};
use syscall_defs::events::keys::KeyCode;
use syscall_defs::events::{Event, EventType};
use syscall_defs::poll::{PollEventFlags, PollFd};

pub struct Input {
    orig_termios: libc::termios,
    kbd: File,
    mouse: File,

    key_queue: [(bool, u8); 16],
    write_pos: usize,
    read_pos: usize,

    mouse_has_data: bool,
    mouse_buttons: (bool, bool, bool),
    mouse_relx: i32,
    mouse_rely: i32,
}

impl Input {
    pub fn new() -> Input {
        let mut termios = MaybeUninit::<libc::termios>::uninit();

        unsafe {
            libc::tcgetattr(0, termios.as_mut_ptr());
        }

        let termios = unsafe { termios.assume_init() };

        let kbd = File::open("/dev/kbd").unwrap();
        let mouse = File::open("/dev/mouse").unwrap();

        let input = Input {
            orig_termios: termios,

            kbd,
            mouse,

            key_queue: [(false, 0); 16],
            write_pos: 0,
            read_pos: 0,

            mouse_has_data: false,
            mouse_buttons: (false, false, false),
            mouse_relx: 0,
            mouse_rely: 0,
        };

        input.enable_raw_mode();

        input
    }

    fn read_event(file: &mut File) -> Event {
        let mut event = MaybeUninit::<Event>::uninit();

        syscall_user::read(file.as_raw_fd() as usize, unsafe {
            std::mem::transmute::<&mut MaybeUninit<Event>, &mut [u8; core::mem::size_of::<Event>()]>(&mut event)
        }).unwrap();

        unsafe { event.assume_init() }
    }

    pub fn poll(&mut self) {
        let mut to_poll = [
            PollFd::new(self.kbd.as_raw_fd(), PollEventFlags::READ),
            PollFd::new(self.mouse.as_raw_fd(), PollEventFlags::READ),
        ];

        loop {
            let res = syscall_user::poll(&mut to_poll, 0).unwrap();

            if res == 0 {
                break;
            }

            if to_poll[0].revents.contains(PollEventFlags::READ) {
                self.handle_keyboard();
            }

            if to_poll[1].revents.contains(PollEventFlags::READ) {
                self.handle_mouse();
            }
        }
    }

    fn handle_mouse(&mut self) {
        let event = Self::read_event(&mut self.mouse);

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

    fn handle_keyboard(&mut self) {
        let event = Self::read_event(&mut self.kbd);

        if event.val == 2 {
            // ignore repeat keys
            return;
        }

        self.add_to_queue(event.val == 0, unsafe { std::mem::transmute(event.code) });
    }

    fn add_to_queue(&mut self, pressed: bool, key: KeyCode) {
        let key = Input::conv_to_doomkey(key);

        self.key_queue[self.write_pos] = (pressed, key);
        self.write_pos = (self.write_pos + 1) % self.key_queue.len();
    }

    pub fn get_key(&mut self) -> Option<(bool, u8)> {
        if self.write_pos == self.read_pos {
            return None;
        }

        let ret = self.key_queue[self.read_pos];
        self.read_pos = (self.read_pos + 1) % self.key_queue.len();

        Some(ret)
    }

    pub fn get_mouse(&mut self) -> Option<((bool, bool, bool), i32, i32)> {
        if !self.mouse_has_data {
            return None;
        }

        self.mouse_has_data = false;

        let ret = Some((self.mouse_buttons, self.mouse_relx, self.mouse_rely));

        self.mouse_relx = 0;
        self.mouse_rely = 0;

        ret
    }

    pub fn quit(&self) {
        self.disable_raw_mode();
    }

    fn enable_raw_mode(&self) {
        let mut new = self.orig_termios;

        unsafe {
            libc::cfmakeraw(&raw mut new);
            libc::tcsetattr(0, libc::TCSAFLUSH, &raw const new);
        }
    }

    fn disable_raw_mode(&self) {
        unsafe {
            libc::tcsetattr(0, libc::TCSAFLUSH, &raw const self.orig_termios);
        }
    }

    fn conv_to_doomkey(key: KeyCode) -> u8 {
        match key {
            KeyCode::KEY_ENTER => crate::keys::KEY_ENTER,
            KeyCode::KEY_ESC => crate::keys::KEY_ESCAPE,
            KeyCode::KEY_LEFT => crate::keys::KEY_LEFTARROW,
            KeyCode::KEY_RIGHT => crate::keys::KEY_RIGHTARROW,
            KeyCode::KEY_W => crate::keys::KEY_UPARROW,
            KeyCode::KEY_S => crate::keys::KEY_DOWNARROW,
            KeyCode::KEY_LEFTCTRL => crate::keys::KEY_FIRE,
            KeyCode::KEY_SPACE => crate::keys::KEY_USE,
            KeyCode::KEY_A => crate::keys::KEY_STRAFE_L,
            KeyCode::KEY_D => crate::keys::KEY_STRAFE_R,
            KeyCode::KEY_LEFTSHIFT => crate::keys::KEY_RSHIFT,

            KeyCode::KEY_Q => 'q' as u8,
            KeyCode::KEY_E => 'e' as u8,
            KeyCode::KEY_R => 'r' as u8,
            KeyCode::KEY_T => 't' as u8,
            KeyCode::KEY_Y => 'y' as u8,
            KeyCode::KEY_U => 'u' as u8,
            KeyCode::KEY_I => 'i' as u8,
            KeyCode::KEY_O => 'o' as u8,
            KeyCode::KEY_P => 'p' as u8,

            KeyCode::KEY_F => 'f' as u8,
            KeyCode::KEY_G => 'g' as u8,
            KeyCode::KEY_H => 'h' as u8,
            KeyCode::KEY_J => 'j' as u8,
            KeyCode::KEY_K => 'k' as u8,
            KeyCode::KEY_L => 'l' as u8,

            KeyCode::KEY_Z => 'z' as u8,
            KeyCode::KEY_X => 'x' as u8,
            KeyCode::KEY_C => 'c' as u8,
            KeyCode::KEY_V => 'v' as u8,
            KeyCode::KEY_B => 'b' as u8,
            KeyCode::KEY_N => 'n' as u8,
            KeyCode::KEY_M => 'm' as u8,

            _ => 0,
        }
    }
}
