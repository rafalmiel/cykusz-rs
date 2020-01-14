use crate::drivers::ps2::PS2Controller;
use crate::drivers::ps2::register_controller;

struct PS2 {}

impl PS2Controller for PS2 {
    fn write_data(&self, _byte: u8) {
        unimplemented!();
    }

    fn read_status(&self) -> u8 {
        unimplemented!()
    }

    fn write_cmd(&self, _byte: u8) {
        unimplemented!();
    }
}

static PS : PS2 = PS2{};

pub(crate) fn init() {
    register_controller(&PS);
}
