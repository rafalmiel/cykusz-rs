use arch::raw::cpuio::{Port, UnsafePort};

use spin::Mutex;

pub static PIC: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(0x20, 0x28) });

// Cmd sent to begin PIC initialization
const CMD_INIT: u8 = 0x11;

// Cmd sent to acknowledge an interrupt
const CMD_END_OF_INTERRUPT: u8 = 0x20;

// The mode in which we want to run PIC
const MODE_8086: u8 = 0x01;

const CMD_READ_ISR: u8 = 0x0B;

struct Pic {
    offset: u8,
    command: UnsafePort<u8>,
    data: UnsafePort<u8>,
}

impl Pic {
    fn handles_interrupt(&self, int_id: u8) -> bool {
        self.offset <= int_id && (int_id < self.offset + 8)
    }

    unsafe fn end_of_interrupt(&mut self) {
        //println!("EOI 0x{:x}", self.offset);
        self.command.write(CMD_END_OF_INTERRUPT);
    }
}

pub struct ChainedPics {
    pics: [Pic; 2],
}

impl ChainedPics {
    pub const unsafe fn new(offset1: u8, offset2: u8) -> ChainedPics {
        ChainedPics {
            pics: [Pic {
                       offset: offset1,
                       command: UnsafePort::new(0x20),
                       data: UnsafePort::new(0x21),
                   },
                   Pic {
                       offset: offset2,
                       command: UnsafePort::new(0xA0),
                       data: UnsafePort::new(0xA1),
                   }],
        }
    }

    unsafe fn configure(&mut self) {
        let mut wait_port: Port<u8> = Port::new(0x80);
        let mut wait = || wait_port.write(0);

        let saved_mask1 = self.pics[0].data.read();
        let saved_mask2 = self.pics[1].data.read();

        // starts the initialization sequence (in cascade mode)
        self.pics[0].command.write(CMD_INIT);
        wait();
        self.pics[1].command.write(CMD_INIT);
        wait();

        // Master PIC vector offset
        self.pics[0].data.write(self.pics[0].offset);
        wait();
        // Slave PIC vector offset
        self.pics[1].data.write(self.pics[1].offset);
        wait();

        // tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
        self.pics[0].data.write(4);
        wait();
        // tell Slave PIC its cascade identity (0000 0010)
        self.pics[1].data.write(2);
        wait();

        self.pics[0].data.write(MODE_8086);
        wait();
        self.pics[1].data.write(MODE_8086);
        wait();

        self.pics[0].data.write(saved_mask1 | 0b00000001);//disable timer?
        self.pics[1].data.write(saved_mask2);
    }

    pub fn disable(&mut self) {
        unsafe {
            let mut wait_port: Port<u8> = Port::new(0x80);
            let mut wait = || wait_port.write(0);
            self.pics[0].data.write(0xFF);
            wait();
            self.pics[1].data.write(0xFF);
            wait();
        }
    }

    pub fn init(&mut self) {
        unsafe {
            self.configure();
        }
    }

    pub fn mask_int(&mut self, irq: u8, masked: bool) {
        let (irqline, port) =
            if irq < 8 { (irq, 0) } else { (irq - 8, 1) };

        let val = if masked {
            unsafe {
                self.pics[port].data.read() | (1 << irqline)
            }
        } else {
            unsafe {
                self.pics[port].data.read() & !(1 << irqline)
            }
        };

        unsafe {
            self.pics[port].data.write(val);
        }
    }

    pub fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        self.pics.iter().any(|p| p.handles_interrupt(interrupt_id))
    }

    unsafe fn get_irq_reg(&mut self, cmd: u8) -> u16 {
        self.pics[1].command.write(cmd);
        self.pics[0].command.write(cmd);

        return ((self.pics[1].command.read() as u16) << 8) | (self.pics[0].command.read() as u16);
    }

    pub fn get_isr(&mut self) -> u16 {
        unsafe {
            self.get_irq_reg(CMD_READ_ISR)
        }
    }

    fn is_pic0_pic1_active(&mut self) -> (bool, bool) {
        let isr = self.get_isr();

        ((isr & 0xFF) > 0, (isr >> 8) > 0)
    }

    pub fn notify_end_of_interrupt(&mut self) {
        let (p0, p1) = self.is_pic0_pic1_active();
        unsafe {
            if p0 || p1 {
                if p1 {
                    self.pics[1].end_of_interrupt();
                }

                self.pics[0].end_of_interrupt();
            }
        }
    }

}

pub fn init() {
    PIC.lock().init();
}

pub fn disable() {
    PIC.lock().disable();
}
