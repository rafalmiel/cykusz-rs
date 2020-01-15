use crate::drivers::ps2::{PS2Controller, Command, StatusFlags};
use crate::drivers::ps2::register_controller;
use crate::arch::raw::cpuio::Port;
use crate::kernel::sync::Mutex;

struct I8042PS2Controller {
    data: Mutex<Port<u8>>,
    status: Mutex<Port<u8>>,
    command: Mutex<Port<u8>>,
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

static PS : I8042PS2Controller = unsafe {
    I8042PS2Controller {
        data: Mutex::new(Port::<u8>::new(0x60)),
        status: Mutex::new(Port::<u8>::new(0x64)),
        command: Mutex::new(Port::<u8>::new(0x64)),
    }
};

pub(crate) fn init() {
    register_controller(&PS);
}
