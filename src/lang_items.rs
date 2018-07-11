use core::panic::PanicInfo;
use core::alloc::Layout;

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}


#[cfg(not(test))]
#[no_mangle]
#[lang = "panic_impl"]
pub fn panic_impl(_pi: &PanicInfo) -> ! {
    loop{}
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[lang = "oom"]
fn oom(layout: Layout) -> ! {
    println!("Out of memory! {:?}", layout);
    loop {}
}
