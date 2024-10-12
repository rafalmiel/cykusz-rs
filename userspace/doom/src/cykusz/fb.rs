use crate::DoomScreen;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use syscall_defs::{MMapFlags, MMapProt};

#[derive(Debug)]
pub struct Fb {
    mem: Option<&'static mut [u32]>,
    #[allow(unused)]
    width: usize,
    #[allow(unused)]
    height: usize,
    pitch: usize,

    doom_screen: DoomScreen,
}

impl Fb {
    pub fn new(doom_screen: DoomScreen) -> Result<Fb, ExitCode> {
        let fb = File::open("/dev/fb").map_err(|_| ExitCode::FAILURE)?;

        let mut fb_info = syscall_defs::ioctl::fb::FbInfo::default();

        syscall_user::ioctl(
            fb.as_raw_fd() as usize,
            syscall_defs::ioctl::fb::GFBINFO,
            (&raw mut fb_info) as usize,
        )
        .map_err(|_| ExitCode::FAILURE)?;

        let map = syscall_user::mmap(
            None,
            fb_info.pitch as usize * fb_info.height as usize,
            MMapProt::PROT_READ | MMapProt::PROT_WRITE,
            MMapFlags::MAP_SHARED,
            Some(fb.as_raw_fd() as usize),
            0,
        )
        .map_err(|_| ExitCode::FAILURE)?;

        let fb = Ok(Fb {
            mem: Some(unsafe {
                std::slice::from_raw_parts_mut(
                    map as *mut u32,
                    fb_info.pitch as usize / 4 * fb_info.height as usize,
                )
            }),
            width: fb_info.width as usize,
            height: fb_info.height as usize,
            pitch: fb_info.pitch as usize / 4,
            doom_screen,
        });

        fb
    }

    pub fn flip(&mut self) {
        if let Some(mem) = self.mem.as_mut() {
            for i in 0..self.doom_screen.height {
                mem[i * self.pitch..i * self.pitch + self.doom_screen.width].copy_from_slice(
                    &self.doom_screen.map[i * self.doom_screen.width
                        ..i * self.doom_screen.width + self.doom_screen.width],
                )
            }
        }
    }
}
