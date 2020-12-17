use crate::arch::idt;
use crate::arch::int;
use crate::arch::raw::cpuio;

pub fn init() {
    int::set_irq_dest(8, 40);
    idt::add_shared_irq_handler(40, rtc_handler);

    int::mask_int(8, false);

    int::disable();

    unsafe {
        let mut sel = cpuio::UnsafePort::<u8>::new(0x70);
        let mut cmd = cpuio::UnsafePort::<u8>::new(0x71);

        sel.write(0x8B);
        let prev = cmd.read();

        sel.write(0x8B);
        cmd.write(prev | 0x40);

        sel.write(0x8A);
        let prev = cmd.read();

        sel.write(0x8A);
        cmd.write((prev & 0xF0) | 0xF);
    }

    int::enable();
}

fn eoi() {
    unsafe {
        let mut sel = cpuio::UnsafePort::<u8>::new(0x70);
        let mut cmd = cpuio::UnsafePort::<u8>::new(0x71);

        sel.write(0x0C);
        cmd.read();
    }
}

fn rtc_handler() -> bool {
    eoi();
    true
}
