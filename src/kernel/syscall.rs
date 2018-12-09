pub fn init() {
    ::arch::syscall::init();
}

#[thread_local]
static mut CNT: usize = 0;

pub fn syscall_handler(_num: u32) {
    unsafe {
        CNT += 1;
    }
    //println!("U {}: 0x{:x}", unsafe {::CPU_ID}, unsafe {::arch::raw::ctrlregs::cr3()});
    print!("S({} {:10}),", unsafe {::CPU_ID}, unsafe {CNT});
}