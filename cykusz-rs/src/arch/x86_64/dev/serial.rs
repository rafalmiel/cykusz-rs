use spin::Once;

use crate::arch::x86_64::raw::cpuio::Port;
use crate::kernel::sync::{Spin, SpinGuard};

struct Serial {
    data: Port<u8>,
    line_cmd: Port<u8>,
    line_status: Port<u8>,
}

const SERIAL_COM1_BASE: u16 = 0x3F8;
const SERIAL_LINE_ENABLE_DLAB: u8 = 0x80;

impl Serial {
    unsafe fn new(base: u16) -> Serial {
        Serial {
            data: Port::new(base),
            line_cmd: Port::new(base + 3),
            line_status: Port::new(base + 5),
        }
    }

    fn configure_baud_rate(&mut self, divisor: u16) {
        self.line_cmd.write(SERIAL_LINE_ENABLE_DLAB);
        self.data.write((divisor >> 8) as u8 & 0xFF);
        self.data.write((divisor as u8) & 0xFF);
    }

    fn configure_line(&mut self) {
        self.line_cmd.write(0x03);
    }

    fn is_tx_fifo_empty(&mut self) -> bool {
        (self.line_status.read() & 0x20) > 0
    }

    fn write(&mut self, data: &str) {
        for c in data.chars() {
            while !self.is_tx_fifo_empty() {}

            self.data.write(c as u8);
        }
    }

    fn init(&mut self) {
        self.configure_baud_rate(1);
        self.configure_line();
    }
}

static SERIAL: Once<Spin<Serial>> = Once::new();

pub fn init() {
    SERIAL.call_once(|| {
        let mut s = unsafe { Serial::new(SERIAL_COM1_BASE) };

        s.init();

        Spin::new(s)
    });
}

fn serial<'a>() -> SpinGuard<'a, Serial> {
    SERIAL.r#try().unwrap().lock_irq()
}

pub fn write(s: &str) {
    let mut serial = serial();

    serial.write(s);
}
