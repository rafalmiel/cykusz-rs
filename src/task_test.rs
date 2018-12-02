use kernel::mm::*;

const WORK_COUNT: usize = 0x1000000;
const ITERS: usize = <usize>::max_value();

fn dummy_work() {
    let a = &3 as *const i32;

    // Dummy work
    for _ in 1..WORK_COUNT {
        unsafe {
            let _ = a.read_volatile();
        }
    }

}

fn task() {
    let id = unsafe {::CPU_ID };
    for i in 0..ITERS {
        //println!("K {}: 0x{:x}", unsafe {::CPU_ID}, unsafe {::arch::raw::ctrlregs::cr3()});
        print!("K({} {:10}),", id, i);

        dummy_work();
    }
}

pub fn start(user_program: MappedAddr, user_program_size: usize) {
    ::kernel::sched::create_task(task);
    ::kernel::sched::create_user_task(
        user_program, user_program_size,
        0x60000
    );
}
