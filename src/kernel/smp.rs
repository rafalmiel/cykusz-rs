use core::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};

static SMP_INITIALISED: AtomicBool = ATOMIC_BOOL_INIT;

pub fn init() {
    ::arch::smp::init();
}

pub fn start() {
    ::arch::smp::start();

    mark_smp_initialized();
}

pub fn cpu_count() -> usize {
    ::arch::smp::cpu_count()
}

pub fn notify_ap_ready() {
    ::arch::smp::notify_ap_ready();

    // Waiting for all CPUs to be ready
    while SMP_INITIALISED.load(Ordering::SeqCst) == false {
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
