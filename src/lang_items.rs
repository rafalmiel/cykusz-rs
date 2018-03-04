#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(not(test))]
#[no_mangle]
#[allow(unused_variables)]
#[lang = "panic_fmt"]
pub extern "C" fn panic_fmt(fmt: ::core::fmt::Arguments, file: &str, line: u32) -> ! {
    println!("");
    println!("");
    println!("PANIC in {} at line {}:", file, line);
    println!("    {}", fmt);

    loop {}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}