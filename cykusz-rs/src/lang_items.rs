use core::panic::PanicInfo;

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(not(test))]
#[no_mangle]
#[lang = "panic_impl"]
pub fn panic_impl(pi: &PanicInfo) -> ! {
    println!("PANIC: {:?}", pi);
    logln!("PANIC: {:?}", pi);
    loop {}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
