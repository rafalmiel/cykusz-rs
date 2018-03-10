use arch::raw::cpuio::{Port, UnsafePort};

// Cmd sent to begin PIC initialization
const CMD_INIT: u8 = 0x11;

// Cmd sent to acknowledge an interrupt
const CMD_END_OF_INTERRUPT: u8 = 0x20;

// The mode in which we want to run PIC
const MODE_8086: u8 = 0x01;

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
    pitCh0: UnsafePort<u8>,
    pitMC: UnsafePort<u8>
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
            pitCh0: UnsafePort::new(0x40),
            pitMC : UnsafePort::new(0x43)
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

        self.pics[0].data.write(saved_mask1);//disable timer?
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

    fn handles_interrupt(&self, interrupt_id: u8) -> bool {
        self.pics.iter().any(|p| p.handles_interrupt(interrupt_id))
    }

    pub unsafe fn notify_end_of_interrupt(&mut self, int_id: u8) {
        if self.handles_interrupt(int_id) {
            if self.pics[1].handles_interrupt(int_id) {
            }

            self.pics[1].end_of_interrupt();
            self.pics[0].end_of_interrupt();
        }
    }

}
