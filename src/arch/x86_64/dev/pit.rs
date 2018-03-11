use arch::raw::cpuio::{Port, UnsafePort};

pub struct Pit {
    pitCh0: UnsafePort<u8>,
    pitMC: UnsafePort<u8>,

    //Each tick increments every 1ms
    ticks: u64,
}

impl Pit {
    pub const fn new() -> Pit {
        Pit {
            pitCh0: unsafe { UnsafePort::new(0x40) },
            pitMC : unsafe { UnsafePort::new(0x43) },
            ticks: 0
        }
    }

    pub fn init(&mut self) {
        self.init_timer(10);
    }


    // Supports from 1 to 50ms
    fn init_timer(&mut self, ms: u16) {
        let hz: u16 = 1000u16 / ms;
        let divisor: u16 = ( 1193182u32 / hz as u32 ) as u16;

        unsafe {
            self.pitMC.write(0x36);
        }

        let l: u8 = (divisor & 0xFF) as u8;
        let h: u8 = ((divisor >> 8) & 0xFF) as u8;

        unsafe {
            self.pitCh0.write(l);
            self.pitCh0.write(h);
        }
    }
}
