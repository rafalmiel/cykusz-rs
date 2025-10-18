#![allow(non_snake_case)]
#![allow(unused_variables)]

use acpica::*;

use crate::kernel::mm::*;

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsAllocate(Size: ACPI_SIZE) -> *mut ::core::ffi::c_void {
    let a = crate::kernel::mm::heap::allocate(Size as usize + core::mem::size_of::<usize>())
        .unwrap() as *mut usize;
    unsafe {
        *a = Size as usize;
    }

    return unsafe { a.offset(1) } as *mut ::core::ffi::c_void;
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsFree(Memory: *mut ::core::ffi::c_void) {
    let a = Memory as *mut usize;
    let (ptr, size) = unsafe {
        let s = a.offset(-1).read();
        (a.offset(-1), s + core::mem::size_of::<usize>())
    };

    crate::kernel::mm::heap::deallocate(ptr as *mut u8, size);
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsMapMemory(
    Where: ACPI_PHYSICAL_ADDRESS,
    Length: ACPI_SIZE,
) -> *mut ::core::ffi::c_void {
    PhysAddr(Where as usize).to_mapped().0 as *mut core::ffi::c_void
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsUnmapMemory(LogicalAddress: *mut ::core::ffi::c_void, Size: ACPI_SIZE) {}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsGetPhysicalAddress(
    LogicalAddress: *mut ::core::ffi::c_void,
    PhysicalAddress: *mut ACPI_PHYSICAL_ADDRESS,
) -> ACPI_STATUS {
    unsafe {
        (PhysicalAddress as *mut isize)
            .write(MappedAddr(LogicalAddress as usize).to_phys().0 as isize)
    }

    AE_OK
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsReadable(Pointer: *mut ::core::ffi::c_void, Length: ACPI_SIZE) -> BOOLEAN {
    true
}

#[unsafe(no_mangle)]
#[linkage = "external"]
extern "C" fn AcpiOsWritable(Pointer: *mut ::core::ffi::c_void, Length: ACPI_SIZE) -> BOOLEAN {
    true
}
