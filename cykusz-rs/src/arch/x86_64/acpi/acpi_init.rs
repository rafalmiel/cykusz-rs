use core::ptr::*;

use acpica::*;

use crate::arch::x86_64::raw::cpuio::Port;
use crate::kernel::timer::busy_sleep;

const ACPI_ROOT_OBJECT: *mut core::ffi::c_void = 0xFFFFFFFFFFFFFFFF as *mut core::ffi::c_void;

#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "C" fn embedded_ctl(
    Function: acpica::UINT32,
    Address: acpica::ACPI_PHYSICAL_ADDRESS,
    BitWidth: acpica::UINT32,
    Value: *mut acpica::UINT64,
    _HandlerContext: *mut ::core::ffi::c_void,
    _RegionContext: *mut ::core::ffi::c_void,
) -> acpica::ACPI_STATUS {
    let mut data = Port::<u8>::new(62);
    let mut cmd = Port::<u8>::new(66);

    let wait_for = |mask, value| -> bool {
        let mut cmd = Port::<u8>::new(66);
        for _ in 0..1000 {
            if cmd.read() & mask == value {
                return true;
            } else {
                busy_sleep(100000);
            }
        }

        println!("EC: Wait Failed");
        return false;
    };

    let mut global_lock_handle = 0;
    assert_eq!(
        acpica::AcpiAcquireGlobalLock(u16::max_value(), &mut global_lock_handle as *mut u32),
        acpica::AE_OK
    );

    if BitWidth != 8 {
        panic!("Unsupported BitWidth {}", BitWidth);
    }

    if Function == 0 {
        //Read
        if !wait_for(0b10, 0) {
            *Value = 0xFF;
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        cmd.write(0x80);
        if !wait_for(0b10, 0) {
            *Value = 0xFF;
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        data.write(Address as u8);
        if !wait_for(0b1, 0b1) {
            *Value = 0xFF;
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        *Value = data.read() as u64;
    } else {
        //Write
        if !wait_for(0b10, 0) {
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        cmd.write(0x81);
        if !wait_for(0b10, 0) {
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        data.write(Address as u8);
        if !wait_for(0b10, 0) {
            acpica::AcpiReleaseGlobalLock(global_lock_handle);
            return AE_OK;
        }
        data.write(*Value as u8);
    }

    acpica::AcpiReleaseGlobalLock(global_lock_handle);
    acpica::AE_OK
}

fn call_pic1() {
    let mut arg = ACPI_OBJECT {
        Type: ACPI_TYPE_INTEGER,
    };
    arg.Integer.Value = 1;

    let mut arg_list = ACPI_OBJECT_LIST {
        Count: 1,
        Pointer: &mut arg as *mut ACPI_OBJECT,
    };

    let mut res = ACPI_BUFFER {
        Length: -1isize as ACPI_SIZE,
        Pointer: null_mut(),
    };

    let status = unsafe {
        AcpiEvaluateObject(
            null_mut(),
            acpi_str(b"\\_PIC\0"),
            &mut arg_list as *mut ACPI_OBJECT_LIST,
            &mut res as *mut ACPI_BUFFER,
        )
    };

    if status != AE_OK {
        println!("PIC Execution failed");
    }
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "C" fn embedded_ctl_setup(
    _RegionHandle: acpica::ACPI_HANDLE,
    _Function: acpica::UINT32,
    _HandlerContext: *mut ::core::ffi::c_void,
    _RegionContext: *mut *mut ::core::ffi::c_void,
) -> acpica::ACPI_STATUS {
    acpica::AE_OK
}

pub fn init() {
    unsafe {
        assert_eq!(AcpiInitializeSubsystem(), AE_OK);
        assert_eq!(
            AcpiInitializeTables(core::ptr::null_mut(), 16, false),
            AE_OK
        );
        assert_eq!(AcpiLoadTables(), AE_OK);
        assert_eq!(AcpiEnableSubsystem(0), AE_OK);

        assert_eq!(
            AcpiInstallAddressSpaceHandler(
                ACPI_ROOT_OBJECT,
                3,
                Some(embedded_ctl),
                Some(embedded_ctl_setup),
                null_mut()
            ),
            AE_OK
        );

        assert_eq!(AcpiInitializeObjects(0), AE_OK);
    }

    call_pic1();

    pci_routing();
    loop{}
}

fn acpi_str(v: &[u8]) -> *mut i8 {
    v.as_ptr() as *mut i8
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "C" fn get_irq_resource(
    Resource: *mut ACPI_RESOURCE,
    Context: *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    let res = &*Resource;
    let tbl = &*(Context as *mut acpi_pci_routing_table);

    match res.Type {
        ACPI_RESOURCE_TYPE_IRQ => {
            //println!("I Count {}", res.Data.Irq.InterruptCount);
            println!(
                "add r irq {} {} {}",
                tbl.Address >> 16,
                tbl.Pin as u8,
                *res.Data
                    .Irq
                    .Interrupts
                    .as_ptr()
                    .offset(tbl.SourceIndex as u8 as isize)
            );
        }
        ACPI_RESOURCE_TYPE_EXTENDED_IRQ => {
            //println!("I Count {}", res.Data.ExtendedIrq.InterruptCount);
            println!(
                "add r2 irq {} {} {}",
                tbl.Address >> 16,
                tbl.Pin as u8,
                *res.Data
                    .ExtendedIrq
                    .Interrupts
                    .as_ptr()
                    .offset(tbl.SourceIndex as u8 as isize)
            );
        }
        _ => {}
    }

    AE_OK
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "C" fn pci_add_root_dev(
    Object: ACPI_HANDLE,
    _NestingLevel: UINT32,
    _Context: *mut ::core::ffi::c_void,
    _ReturnValue: *mut *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    println!("Found PCI root bridge");

    let mut buf: ACPI_BUFFER = ACPI_BUFFER {
        Length: -1isize as ACPI_SIZE, // ACPI_ALLOCATE_BUFFER
        Pointer: null_mut(),
    };

    assert_eq!(
        AcpiGetIrqRoutingTable(Object, &mut buf as *mut ACPI_BUFFER),
        AE_OK
    );

    println!("{:?}", buf);

    let mut tbl = &mut *(buf.Pointer as *mut acpi_pci_routing_table);

    while tbl.Length != 0 {
        //println!("{:?}", tbl);

        if tbl.Source[0] == 0 {
            println!(
                "add irq {} {} {}",
                tbl.Address >> 16,
                tbl.Pin,
                tbl.SourceIndex
            );
        } else {
            let mut src_handle: ACPI_HANDLE = null_mut();

            assert_eq!(
                AcpiGetHandle(
                    Object,
                    tbl.Source.as_mut_ptr(),
                    &mut src_handle as *mut ACPI_HANDLE
                ),
                AE_OK
            );

            assert_eq!(
                AcpiWalkResources(
                    src_handle,
                    acpi_str(b"_CRS\0"),
                    Some(get_irq_resource),
                    tbl as *mut _ as *mut core::ffi::c_void
                ),
                AE_OK
            );
        }

        buf.Pointer = buf.Pointer.offset(tbl.Length as isize);
        tbl = &mut *(buf.Pointer as *mut acpi_pci_routing_table);

        crate::bochs();
    }
    AE_OK
}
pub fn pci_routing() {
    println!("PCI Routing:");
    unsafe {
        assert_eq!(
            AcpiGetDevices(
                acpi_str(b"PNP0A03\0"),
                Some(pci_add_root_dev),
                null_mut(),
                null_mut()
            ),
            AE_OK
        );
    }
}
