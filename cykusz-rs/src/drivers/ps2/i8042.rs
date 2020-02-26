use crate::arch::raw::cpuio::Port;
use crate::drivers::ps2::register_controller;
use crate::drivers::ps2::{Command, PS2Controller, StatusFlags};
use crate::kernel::sync::Spin;

struct I8042PS2Controller {
    data: Spin<Port<u8>>,
    status: Spin<Port<u8>>,
    command: Spin<Port<u8>>,
}

impl PS2Controller for I8042PS2Controller {
    fn write(&self, byte: u8) {
        self.data.lock().write(byte);
    }

    fn read(&self) -> u8 {
        self.data.lock().read()
    }

    fn command(&self, byte: Command) {
        self.command.lock().write(byte as u8);
    }

    fn status(&self) -> StatusFlags {
        StatusFlags::from_bits_truncate(self.status.lock().read())
    }
}

static PS: I8042PS2Controller = unsafe {
    I8042PS2Controller {
        data: Spin::new(Port::<u8>::new(0x60)),
        status: Spin::new(Port::<u8>::new(0x64)),
        command: Spin::new(Port::<u8>::new(0x64)),
    }
};

pub(crate) fn init() {
    register_controller(&PS);
}
