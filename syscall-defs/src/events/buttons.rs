#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ButtonCode {
    BTN_MOUSE = 0x100,
    BTN_LEFT = 0x110,
    BTN_RIGHT = 0x111,
    BTN_MIDDLE = 0x112,
    BTN_SIDE = 0x113,
    BTN_EXTRA = 0x114,
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RelCode {
    REL_X = 0x00,
    REL_Y = 0x01,
}
