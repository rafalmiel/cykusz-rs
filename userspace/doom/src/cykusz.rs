mod audio;
mod fb;
mod input;

use crate::DoomScreen;
use std::process::ExitCode;

pub struct CykuszDoom {
    fb: fb::Fb,
    input: input::Input,
    audio: audio::Audio,
}

impl CykuszDoom {
    pub fn new() -> Result<CykuszDoom, ExitCode> {
        Ok(CykuszDoom {
            fb: fb::Fb::new()?,
            input: input::Input::new()?,
            audio: audio::Audio::new()?,
        })
    }

    pub fn get_ticks_ms(&self) -> u32 {
        ((unsafe { syscall_user::syscall0(syscall_defs::SYS_TICKSNS).unwrap() }) / 1_000_000) as u32
    }

    pub fn sleep_ms(&self, ms: u32) {
        syscall_user::sleep(ms as usize).unwrap();
    }

    pub fn draw_frame(&mut self, out: &mut DoomScreen) {
        self.input.poll();
        self.fb.flip(out);
    }

    pub fn get_key(&mut self) -> Option<(bool, u8)> {
        self.input.get_key()
    }

    pub fn get_mouse(&mut self) -> Option<((bool, bool, bool), i32, i32)> {
        self.input.get_mouse()
    }

    pub fn quit(&self) {
        self.input.quit();
    }

    pub fn audio(&mut self) -> &mut audio::Audio {
        &mut self.audio
    }
}
