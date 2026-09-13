#[unsafe(no_mangle)]
pub extern "C" fn on_user_enter() {
    assert!(crate::kernel::int::is_enabled());
    dbgln!(ipi, "on user enter");
    crate::run_deferred_tasks("on_user_enter");
}
