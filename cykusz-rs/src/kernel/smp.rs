use core::sync::atomic::{AtomicBool, Ordering};

static SMP_INITIALISED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    crate::arch::smp::init();
}

pub fn start() {
    crate::arch::smp::start();

    mark_smp_initialized();
}

pub fn cpu_count() -> usize {
    crate::arch::smp::cpu_count()
}

pub fn notify_ap_ready() {
    crate::arch::smp::notify_ap_ready();

    // Waiting for all CPUs to be ready
    while !is_smp_initialised() {
        unsafe {
            asm!("pause"::::"volatile");
        }
    }
}

pub fn is_smp_initialised() -> bool {
    SMP_INITIALISED.load(Ordering::SeqCst)
}

fn mark_smp_initialized() {
    SMP_INITIALISED.store(true, Ordering::SeqCst);
}
