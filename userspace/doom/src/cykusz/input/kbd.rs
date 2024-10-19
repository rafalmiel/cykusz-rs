use crate::cykusz::input::read_event;
use crate::doomgeneric;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use syscall_defs::events::keys::KeyCode;
use syscall_defs::poll::{PollEventFlags, PollFd};

pub struct KeyboardInput {
    file: File,
    kbd_key_queue: [(bool, u8); 16],
    kbd_write_pos: usize,
    kbd_read_pos: usize,
}

impl KeyboardInput {
    pub fn new() -> Result<KeyboardInput, ExitCode> {
        let kbd = File::open("/dev/kbd").map_err(|_| ExitCode::FAILURE)?;
        Ok(KeyboardInput {
            file: kbd,
            kbd_key_queue: [(false, 0); 16],
            kbd_write_pos: 0,
            kbd_read_pos: 0,
        })
    }

    pub fn poll_fd(&self) -> PollFd {
        PollFd::new(self.file.as_raw_fd(), PollEventFlags::READ)
    }

    pub fn handle(&mut self) {
        let event = read_event(&mut self.file);

        if event.val == 2 {
            // ignore repeat keys
            return;
        }

        self.add_to_queue(event.val == 0, unsafe { std::mem::transmute(event.code) });
    }

    fn add_to_queue(&mut self, pressed: bool, key: KeyCode) {
        let key = Self::conv_to_doomkey(key);

        self.kbd_key_queue[self.kbd_write_pos] = (pressed, key);
        self.kbd_write_pos = (self.kbd_write_pos + 1) % self.kbd_key_queue.len();
    }

    pub fn get_key(&mut self) -> Option<(bool, u8)> {
        if self.kbd_write_pos == self.kbd_read_pos {
            return None;
        }

        let ret = self.kbd_key_queue[self.kbd_read_pos];
        self.kbd_read_pos = (self.kbd_read_pos + 1) % self.kbd_key_queue.len();

        Some(ret)
    }

    fn conv_to_doomkey(key: KeyCode) -> u8 {
        match key {
            KeyCode::KEY_ENTER => doomgeneric::KEY_ENTER as u8,
            KeyCode::KEY_ESC => doomgeneric::KEY_ESCAPE as u8,
            KeyCode::KEY_LEFT => doomgeneric::KEY_LEFTARROW as u8,
            KeyCode::KEY_RIGHT => doomgeneric::KEY_RIGHTARROW as u8,
            KeyCode::KEY_UP => doomgeneric::KEY_UPARROW as u8,
            KeyCode::KEY_DOWN => doomgeneric::KEY_DOWNARROW as u8,
            KeyCode::KEY_LEFTCTRL => doomgeneric::KEY_FIRE as u8,
            KeyCode::KEY_SPACE => doomgeneric::KEY_USE as u8,
            KeyCode::KEY_LEFTSHIFT => doomgeneric::KEY_RSHIFT as u8,
            KeyCode::KEY_BACKSPACE => doomgeneric::KEY_BACKSPACE as u8,
            KeyCode::KEY_TAB => doomgeneric::KEY_TAB as u8,
            KeyCode::KEY_MINUS => doomgeneric::KEY_MINUS as u8,
            KeyCode::KEY_EQUAL => doomgeneric::KEY_EQUALS as u8,

            KeyCode::KEY_A => 'a' as u8,
            KeyCode::KEY_B => 'b' as u8,
            KeyCode::KEY_C => 'c' as u8,
            KeyCode::KEY_D => 'd' as u8,
            KeyCode::KEY_E => 'e' as u8,
            KeyCode::KEY_F => 'f' as u8,
            KeyCode::KEY_G => 'g' as u8,
            KeyCode::KEY_H => 'h' as u8,
            KeyCode::KEY_I => 'i' as u8,
            KeyCode::KEY_J => 'j' as u8,
            KeyCode::KEY_K => 'k' as u8,
            KeyCode::KEY_L => 'l' as u8,
            KeyCode::KEY_M => 'm' as u8,
            KeyCode::KEY_N => 'n' as u8,
            KeyCode::KEY_O => 'o' as u8,
            KeyCode::KEY_P => 'p' as u8,
            KeyCode::KEY_Q => 'q' as u8,
            KeyCode::KEY_R => 'r' as u8,
            KeyCode::KEY_S => 's' as u8,
            KeyCode::KEY_T => 't' as u8,
            KeyCode::KEY_U => 'u' as u8,
            KeyCode::KEY_V => 'v' as u8,
            KeyCode::KEY_W => 'w' as u8,
            KeyCode::KEY_X => 'x' as u8,
            KeyCode::KEY_Y => 'y' as u8,
            KeyCode::KEY_Z => 'z' as u8,

            KeyCode::KEY_1 => '1' as u8,
            KeyCode::KEY_2 => '2' as u8,
            KeyCode::KEY_3 => '3' as u8,
            KeyCode::KEY_4 => '4' as u8,
            KeyCode::KEY_5 => '5' as u8,
            KeyCode::KEY_6 => '6' as u8,
            KeyCode::KEY_7 => '7' as u8,
            KeyCode::KEY_8 => '8' as u8,
            KeyCode::KEY_9 => '9' as u8,
            KeyCode::KEY_0 => '0' as u8,

            KeyCode::KEY_F1 => doomgeneric::KEY_F1 as u8,
            KeyCode::KEY_F2 => doomgeneric::KEY_F2 as u8,
            KeyCode::KEY_F3 => doomgeneric::KEY_F3 as u8,
            KeyCode::KEY_F4 => doomgeneric::KEY_F4 as u8,
            KeyCode::KEY_F5 => doomgeneric::KEY_F5 as u8,
            KeyCode::KEY_F6 => doomgeneric::KEY_F6 as u8,
            KeyCode::KEY_F7 => doomgeneric::KEY_F7 as u8,
            KeyCode::KEY_F8 => doomgeneric::KEY_F8 as u8,
            KeyCode::KEY_F9 => doomgeneric::KEY_F9 as u8,
            KeyCode::KEY_F10 => doomgeneric::KEY_F10 as u8,
            KeyCode::KEY_F11 => doomgeneric::KEY_F11 as u8,

            _ => 0,
        }
    }
}
