use arch::raw::cpuio::{UnsafePort};

use arch::int;
use arch::idt;
use arch::raw::idt as ridt;

use kernel::sync::Mutex;

static PIT: Mutex<Pit> = Mutex::new(Pit::new());

pub struct Pit {
    pit_ch0: UnsafePort<u8>,
    pit_mc: UnsafePort<u8>,

    //Each tick increments every 1ms
    ticks: u64,
}

impl Pit {
    pub const fn new() -> Pit {
        Pit {
            pit_ch0: unsafe { UnsafePort::new(0x40) },
            pit_mc : unsafe { UnsafePort::new(0x43) },
            ticks: 0
        }
    }

    pub fn init(&mut self) {
        //self.init_timer(10);
    }

    pub fn init_sleep(&mut self) {
        unsafe {
            //Interrupt on terminal count mode for counter 0
            self.pit_mc.write(0x30);

            //Set count value
            self.pit_ch0.write(0xA9);
            self.pit_ch0.write(0x4);
        }
    }

    pub fn is_sleep_finished(&mut self) -> bool {
        unsafe {
            self.pit_mc.write(0xE2);
            let status = self.pit_ch0.read();

            return (status & 0b1000_0000) != 0;
        }
    }

    // Supports from 1 to 50ms
    #[allow(unused)]
    fn init_timer(&mut self, ms: u16) {
        let hz: u16 = 1000u16 / ms;
        let divisor: u16 = ( 1193182u32 / hz as u32 ) as u16;

        unsafe {
            self.pit_mc.write(0x36);
        }

        let l: u8 = (divisor & 0xFF) as u8;
        let h: u8 = ((divisor >> 8) & 0xFF) as u8;

        unsafe {
            self.pit_ch0.write(l);
            self.pit_ch0.write(h);
        }
    }
}

pub fn init() {
    int::set_irq_dest(0, 32);
    idt::set_handler(32, pit_handler);

    int::mask_int(0, false);

    PIT.lock_irq().init();
}

pub fn disable() {
    int::mask_int(0, true);
}

pub fn enable() {
    int::mask_int(0, false);
}

pub extern "x86-interrupt" fn pit_handler(_frame: &mut ridt::ExceptionStackFrame) {
    let pit = &mut *PIT.lock();
    pit.ticks += 1;
    int::end_of_int();
}

pub fn early_sleep(mut ms: u64) {
    let mut pit = PIT.lock_irq();
    while ms > 0 {
        pit.init_sleep();

        while !pit.is_sleep_finished() {
            unsafe {
                asm!("pause"::::"volatile");
            }
        }
        ms -= 1;
    }
}