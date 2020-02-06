use core::panic::PanicInfo;

#[allow(unused)]
pub fn bochs() {
    unsafe {
        asm!("xchg %bx, %bx");
    }
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    super::main()
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panicked... {:?}", info);
    loop {}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
