pub fn init() {
    ::arch::tls::init();
}

pub fn is_ready() -> bool {
    ::SMP_INITIALISED.load(::core::sync::atomic::Ordering::SeqCst)
}
