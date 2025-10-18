#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

use crate::kernel::timer::busy_sleep;

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsGetThreadId() -> UINT64 {
    crate::kernel::sched::current_id() as i64 + 1
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsExecute(
    Type: ACPI_EXECUTE_TYPE,
    Function: ACPI_OSD_EXEC_CALLBACK,
    Context: *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    println!("AcpiOsExecute: Fun {:p}({:p})", Function.unwrap(), Context);
    crate::kernel::sched::create_param_task(Function.unwrap() as usize, Context as usize);
    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsWaitEventsComplete() {
    unimplemented!()
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsSleep(Milliseconds: UINT64) {
    use crate::kernel::sched::current_task;

    current_task()
        .sleep(Milliseconds as usize * 1_000_000)
        .expect("Unexpected signal in acpica thread");
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsStall(Microseconds: UINT32) {
    busy_sleep(Microseconds as u64 * 1000)
}
