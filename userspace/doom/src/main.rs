#![allow(non_snake_case)]
#![feature(raw_ref_op)]

mod cykusz;
mod keys;

use std::ffi::{c_char, c_int, c_uchar, c_uint, CString};

static mut DOOM: Option<cykusz::CykuszDoom> = None;

#[derive(Debug)]
pub struct DoomScreen {
    map: &'static [u32],
    width: usize,
    height: usize,
}

fn doom<'a>() -> &'a mut cykusz::CykuszDoom {
    unsafe { DOOM.as_mut().unwrap_unchecked() }
}

extern "C" {
    fn doomgeneric_Create(c: c_int, argv: *const *const c_char);
    fn doomgeneric_Tick();

    static DG_ScreenBuffer: *mut u32;
}

fn doomgeneric_screen() -> DoomScreen {
    pub const DOOMGENERIC_RESX: usize = 640;
    pub const DOOMGENERIC_RESY: usize = 400;

    DoomScreen {
        map: unsafe {
            std::slice::from_raw_parts(DG_ScreenBuffer, DOOMGENERIC_RESX * DOOMGENERIC_RESY * 4)
        },
        width: DOOMGENERIC_RESX,
        height: DOOMGENERIC_RESY,
    }
}

#[no_mangle]
extern "C" fn DG_Init() {
    unsafe {
        (&raw mut DOOM).write(Some(cykusz::CykuszDoom::new(doomgeneric_screen()).unwrap()));
    }
}

#[no_mangle]
extern "C" fn DG_DrawFrame() {
    doom().draw_frame()
}

#[no_mangle]
extern "C" fn DG_SleepMs(ms: c_uint) {
    doom().sleep_ms(ms as u32);
}

#[no_mangle]
extern "C" fn DG_GetTicksMs() -> c_uint {
    doom().get_ticks_ms()
}

#[no_mangle]
extern "C" fn DG_GetKey(pressed: *mut c_int, doomkey: *mut c_uchar) -> c_int {
    if let Some((p, key)) = doom().get_key() {
        unsafe {
            pressed.write(if p { 1 } else { 0 });
            doomkey.write(key);
        }

        1
    } else {
        0
    }
}

#[no_mangle]
extern "C" fn DG_GetMouse(buttons: *mut c_int, rel_x: *mut c_int, rel_y: *mut c_int) -> c_int {
    if let Some(((left, right, mid), relx, rely)) = doom().get_mouse() {
        let mut btns: c_int = 0;
        if left {
            btns |= 1 << 0;
        }
        if right {
            btns |= 1 << 1;
        }
        if mid {
            btns |= 1 << 2;
        }

        unsafe {
            buttons.write(btns);
            rel_x.write(relx);
            rel_y.write(rely);
        }

        1
    } else {
        0
    }
}

#[no_mangle]
extern "C" fn DG_SetWindowTitle(_title: *const c_char) {}

extern "C" fn DG_Quit() {
    doom().quit();
}

fn main() {
    unsafe {
        let args = std::env::args()
            .map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();
        // convert the strings to raw pointers
        let c_args = args
            .iter()
            .map(|arg| arg.as_ptr())
            .collect::<Vec<*const c_char>>();

        doomgeneric_Create(c_args.len() as c_int, c_args.as_ptr());

        libc::atexit(DG_Quit);

        loop {
            doomgeneric_Tick();
        }
    }
}
