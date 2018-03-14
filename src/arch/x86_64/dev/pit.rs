use arch::raw::cpuio::{Port, UnsafePort};

use arch::int;
use arch::idt;
use arch::raw::idt as ridt;

use arch::sync::Mutex;

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
        self.init_timer(10);
    }

    // Supports from 1 to 50ms
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
    let remap: u8 = int::get_irq_mapping(0) as u8;
    int::set_irq_dest(remap as u8, 32);
    idt::set_handler(32, pit_handler);

    int::mask_int(remap, false);

    PIT.lock_irq().init();
}

pub fn disable() {
    let remap: u8 = int::get_irq_mapping(0) as u8;
    int::mask_int(remap, true);
}

pub fn enable() {
    let remap: u8 = int::get_irq_mapping(0) as u8;
    int::mask_int(remap, false);
}

pub extern "x86-interrupt" fn pit_handler(_frame: &mut ridt::ExceptionStackFrame) {
    let pit = &mut *PIT.lock();
    pit.ticks += 1;
    int::end_of_int();
}

// Should be used only at early stage
// After enabling multiple cpus or even threads, this function is not thread safe
pub fn early_sleep(ms10: u64) {
    enable();
    let cur = PIT.lock_irq().ticks;
    let dst = cur + ms10;

    loop {
        let pit = PIT.lock_irq();

        if pit.ticks >= dst {
            break;
        }
        unsafe {
            //TODO: Move somewhere else
            asm!("pause")
        }
    }
    disable();
}