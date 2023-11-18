use syscall_defs::events::keys::KeyCode;
macro_rules! init_arr (
    ($a: ident, $([$k: expr, $v: expr]),+) => {
        $($a[$k] = $v;)*
    };
);

pub static MAP_L1: [KeyCode; 256] = {
    let mut arr = [KeyCode::KEY_RESERVED; 256];

    init_arr!(
        arr,
        [0x1c, KeyCode::KEY_A],
        [0x32, KeyCode::KEY_B],
        [0x21, KeyCode::KEY_C],
        [0x23, KeyCode::KEY_D],
        [0x24, KeyCode::KEY_E],
        [0x2b, KeyCode::KEY_F],
        [0x34, KeyCode::KEY_G],
        [0x33, KeyCode::KEY_H],
        [0x43, KeyCode::KEY_I],
        [0x3b, KeyCode::KEY_J],
        [0x42, KeyCode::KEY_K],
        [0x4b, KeyCode::KEY_L],
        [0x3a, KeyCode::KEY_M],
        [0x31, KeyCode::KEY_N],
        [0x44, KeyCode::KEY_O],
        [0x4d, KeyCode::KEY_P],
        [0x15, KeyCode::KEY_Q],
        [0x2d, KeyCode::KEY_R],
        [0x1b, KeyCode::KEY_S],
        [0x2c, KeyCode::KEY_T],
        [0x3c, KeyCode::KEY_U],
        [0x2a, KeyCode::KEY_V],
        [0x1d, KeyCode::KEY_W],
        [0x22, KeyCode::KEY_X],
        [0x35, KeyCode::KEY_Y],
        [0x1a, KeyCode::KEY_Z],
        [0x45, KeyCode::KEY_0],
        [0x16, KeyCode::KEY_1],
        [0x1e, KeyCode::KEY_2],
        [0x26, KeyCode::KEY_3],
        [0x25, KeyCode::KEY_4],
        [0x2e, KeyCode::KEY_5],
        [0x36, KeyCode::KEY_6],
        [0x3d, KeyCode::KEY_7],
        [0x3e, KeyCode::KEY_8],
        [0x46, KeyCode::KEY_9],
        [0x0e, KeyCode::KEY_GRAVE],
        [0x4e, KeyCode::KEY_MINUS],
        [0x55, KeyCode::KEY_EQUAL],
        [0x5d, KeyCode::KEY_BACKSLASH],
        [0x66, KeyCode::KEY_BACKSPACE],
        [0x29, KeyCode::KEY_SPACE],
        [0x0d, KeyCode::KEY_TAB],
        [0x58, KeyCode::KEY_CAPSLOCK],
        [0x12, KeyCode::KEY_LEFTSHIFT],
        [0x14, KeyCode::KEY_LEFTCTRL],
        [0x11, KeyCode::KEY_LEFTALT],
        [0x59, KeyCode::KEY_RIGHTSHIFT],
        [0x5a, KeyCode::KEY_ENTER],
        [0x76, KeyCode::KEY_ESC],
        [0x05, KeyCode::KEY_F1],
        [0x06, KeyCode::KEY_F2],
        [0x04, KeyCode::KEY_F3],
        [0x0c, KeyCode::KEY_F4],
        [0x03, KeyCode::KEY_F5],
        [0x0b, KeyCode::KEY_F6],
        [0x83, KeyCode::KEY_F7],
        [0x0a, KeyCode::KEY_F8],
        [0x01, KeyCode::KEY_F9],
        [0x09, KeyCode::KEY_F10],
        [0x78, KeyCode::KEY_F11],
        [0x07, KeyCode::KEY_F12],
        [0x7e, KeyCode::KEY_SCROLLLOCK],
        [0x54, KeyCode::KEY_LEFTBRACE],
        [0x77, KeyCode::KEY_NUMLOCK],
        [0x7c, KeyCode::KEY_KPASTERISK],
        [0x7b, KeyCode::KEY_KPMINUS],
        [0x79, KeyCode::KEY_KPPLUS],
        [0x71, KeyCode::KEY_KPDOT],
        [0x70, KeyCode::KEY_KP0],
        [0x69, KeyCode::KEY_KP1],
        [0x72, KeyCode::KEY_KP2],
        [0x7a, KeyCode::KEY_KP3],
        [0x6b, KeyCode::KEY_KP4],
        [0x73, KeyCode::KEY_KP5],
        [0x74, KeyCode::KEY_KP6],
        [0x6c, KeyCode::KEY_KP7],
        [0x75, KeyCode::KEY_KP8],
        [0x7d, KeyCode::KEY_KP9],
        [0x5b, KeyCode::KEY_RIGHTBRACE],
        [0x4c, KeyCode::KEY_SEMICOLON],
        [0x52, KeyCode::KEY_APOSTROPHE],
        [0x41, KeyCode::KEY_COMMA],
        [0x49, KeyCode::KEY_DOT],
        [0x4a, KeyCode::KEY_SLASH],
        [0x61, KeyCode::KEY_BACKSLASH]
    );

    arr
};
pub static MAP_L2: [KeyCode; 127] = {
    let mut arr = [KeyCode::KEY_RESERVED; 127];

    init_arr!(
        arr,
        [0x1f, KeyCode::KEY_LEFTMETA],
        [0x14, KeyCode::KEY_RIGHTCTRL],
        [0x27, KeyCode::KEY_RIGHTMETA],
        [0x11, KeyCode::KEY_RIGHTALT],
        [0x2f, KeyCode::KEY_COMPOSE],
        [0x70, KeyCode::KEY_INSERT],
        [0x6c, KeyCode::KEY_HOME],
        [0x7d, KeyCode::KEY_PAGEUP],
        [0x71, KeyCode::KEY_DELETE],
        [0x69, KeyCode::KEY_END],
        [0x7a, KeyCode::KEY_PAGEDOWN],
        [0x75, KeyCode::KEY_UP],
        [0x6b, KeyCode::KEY_LEFT],
        [0x72, KeyCode::KEY_DOWN],
        [0x74, KeyCode::KEY_RIGHT],
        [0x4a, KeyCode::KEY_KPSLASH],
        [0x5a, KeyCode::KEY_KPENTER]
    );

    arr
};

pub fn get(sc: usize, l2: bool) -> KeyCode {
    if !l2 {
        MAP_L1[sc]
    } else {
        MAP_L2[sc]
    }
}
