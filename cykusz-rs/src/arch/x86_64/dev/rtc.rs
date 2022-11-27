use core::sync::atomic::{AtomicI64, Ordering};

use crate::arch::idt;
use crate::arch::int;
use crate::arch::raw::cpuio;
use crate::kernel::sync::IrqGuard;

fn read_naive_date() -> chrono::NaiveDateTime {
    unsafe {
        let mut sel = cpuio::UnsafePort::<u8>::new(0x70);
        let mut cmd = cpuio::UnsafePort::<u8>::new(0x71);

        sel.write(0x0);
        let sec = cmd.read() as u32;

        sel.write(0x2);
        let min = cmd.read() as u32;

        sel.write(0x4);
        let hour = cmd.read() as u32;

        sel.write(0x7);
        let day = cmd.read() as u32;

        sel.write(0x8);
        let mon = cmd.read() as u32;

        sel.write(0x9);
        let year = cmd.read() as i32;

        chrono::NaiveDate::from_ymd_opt(year + 2000, mon, day).unwrap().and_hms_opt(hour, min, sec).unwrap()
    }
}

fn set_date() {
    UNIX_TX.store(read_naive_date().timestamp(), Ordering::SeqCst);
}

pub fn init() {
    int::set_irq_dest(8, 40);
    idt::add_shared_irq_handler(40, rtc_handler);

    int::mask_int(8, false);

    let _ = IrqGuard::new();

    unsafe {
        let mut sel = cpuio::UnsafePort::<u8>::new(0x70);
        let mut cmd = cpuio::UnsafePort::<u8>::new(0x71);

        sel.write(0x8B);
        let prev = cmd.read();

        sel.write(0x8B);
        cmd.write(prev | 0x10 | 0b110); //Enable update interrupt and human date format
    }
}

fn eoi() {
    unsafe {
        let mut sel = cpuio::UnsafePort::<u8>::new(0x70);
        let mut cmd = cpuio::UnsafePort::<u8>::new(0x71);

        sel.write(0x0C);
        cmd.read();
    }
}

static UNIX_TX: AtomicI64 = AtomicI64::new(0);

pub fn get_unix_ts() -> i64 {
    UNIX_TX.load(Ordering::SeqCst)
}

fn rtc_handler() -> bool {
    static mut FIRST_INTERRUPT: bool = true;
    if unsafe { FIRST_INTERRUPT } {
        set_date();

        unsafe {
            FIRST_INTERRUPT = false;
        }
    } else {
        UNIX_TX.fetch_add(1, Ordering::SeqCst);
    }
    eoi();
    true
}
