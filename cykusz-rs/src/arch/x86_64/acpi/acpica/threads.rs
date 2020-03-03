use acpica::*;

use crate::kernel::timer::busy_sleep;

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsGetThreadId() -> UINT64 {
    1
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsExecute(
    Type: ACPI_EXECUTE_TYPE,
    Function: ACPI_OSD_EXEC_CALLBACK,
    Context: *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    unimplemented!()
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsWaitEventsComplete() {
    unimplemented!()
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsSleep(Milliseconds: UINT64) {
    use crate::kernel::sched::current_task;

    current_task().sleep(Milliseconds as usize * 1_000_000);
}

#[no_mangle]
#[linkage = "external"]
extern "C" fn AcpiOsStall(Microseconds: UINT32) {
    busy_sleep(Microseconds as u64 * 1000)
}
