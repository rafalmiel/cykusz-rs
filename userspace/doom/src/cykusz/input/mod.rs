mod kbd;
mod mouse;

use crate::cykusz::input::kbd::KeyboardInput;
use crate::cykusz::input::mouse::MouseInput;
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use syscall_defs::events::Event;
use syscall_defs::poll::PollEventFlags;

pub fn read_event(file: &mut File) -> Event {
    let mut event = MaybeUninit::<Event>::uninit();

    syscall_user::read(file.as_raw_fd() as usize, unsafe {
        std::mem::transmute::<&mut MaybeUninit<Event>, &mut [u8; core::mem::size_of::<Event>()]>(
            &mut event,
        )
    })
    .unwrap();

    unsafe { event.assume_init() }
}

pub struct Input {
    orig_termios: libc::termios,

    keyboard: KeyboardInput,
    mouse: MouseInput,
}

impl Input {
    pub fn new() -> Input {
        let mut termios = MaybeUninit::<libc::termios>::uninit();

        unsafe {
            libc::tcgetattr(0, termios.as_mut_ptr());
        }

        let termios = unsafe { termios.assume_init() };

        let input = Input {
            orig_termios: termios,

            keyboard: KeyboardInput::new(),
            mouse: MouseInput::new(),
        };

        input.enable_raw_mode();

        input
    }

    pub fn poll(&mut self) {
        let mut to_poll = [self.keyboard.poll_fd(), self.mouse.poll_fd()];

        loop {
            let res = syscall_user::poll(&mut to_poll, 0).unwrap();

            if res == 0 {
                break;
            }

            if to_poll[0].revents.contains(PollEventFlags::READ) {
                self.keyboard.handle()
            }

            if to_poll[1].revents.contains(PollEventFlags::READ) {
                self.mouse.handle();
            }
        }
    }

    pub fn get_key(&mut self) -> Option<(bool, u8)> {
        self.keyboard.get_key()
    }

    pub fn get_mouse(&mut self) -> Option<((bool, bool, bool), i32, i32)> {
        self.mouse.get_mouse()
    }

    pub fn quit(&self) {
        self.disable_raw_mode();
    }

    fn enable_raw_mode(&self) {
        let mut new = self.orig_termios;

        unsafe {
            libc::cfmakeraw(&raw mut new);
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw const new);
        }
    }

    fn disable_raw_mode(&self) {
        unsafe {
            libc::tcsetattr(
                libc::STDIN_FILENO,
                libc::TCSAFLUSH,
                &raw const self.orig_termios,
            );
        }
    }
}
