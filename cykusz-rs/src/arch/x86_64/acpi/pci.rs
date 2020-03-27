use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::ptr::null_mut;

use spin::Once;

use acpica::*;

use crate::kernel::sync::{Spin, SpinGuard};

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
unsafe extern "C" fn get_irq_resource(
    Resource: *mut ACPI_RESOURCE,
    Context: *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    let res = &*Resource;
    let data = &*(Context as *mut ResData);

    let tbl = &*data.tbl;
    let bridge = &mut *data.bridge;

    match res.Type {
        ACPI_RESOURCE_TYPE_IRQ => {
            //println!("I Count {}", res.Data.Irq.InterruptCount);
            bridge.add_irq(
                tbl.Address >> 16,
                tbl.Pin as u8,
                *res.Data
                    .Irq
                    .Interrupts
                    .as_ptr()
                    .offset(tbl.SourceIndex as u8 as isize) as u32,
            );
        }
        ACPI_RESOURCE_TYPE_EXTENDED_IRQ => {
            //println!("I Count {}", res.Data.ExtendedIrq.InterruptCount);
            bridge.add_irq(
                tbl.Address >> 16,
                tbl.Pin as u8,
                *res.Data
                    .ExtendedIrq
                    .Interrupts
                    .as_ptr()
                    .offset(tbl.SourceIndex as u8 as isize),
            );
        }
        _ => {}
    }

    AE_OK
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "C" fn add_pci_dev(
    Object: ACPI_HANDLE,
    NestingLevel: UINT32,
    Context: *mut ::core::ffi::c_void,
    ReturnValue: *mut *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    let bridge = &mut *(Context as *mut PciBridge);
    let mut parent: ACPI_HANDLE = null_mut();

    if Object == root_handle() {
        return AE_OK;
    }

    let mut new_bridge = PciBridge::new(Object);

    assert_eq!(
        AcpiGetParent(Object, &mut parent as *mut ACPI_HANDLE),
        AE_OK
    );

    if parent != bridge.acpi_handle {
        return AE_OK;
    }

    if new_bridge.init_irq_routing() {
        let (dev, fun) = new_bridge.init_dev_fun();

        let map = crate::drivers::pci::read_u32(bridge.secondary as u8, dev as u8, fun as u8, 0x18)
            & 0xffff;

        new_bridge.primary = (map & 0xff) as i8 as i32;
        new_bridge.secondary = ((map >> 8) & 0xff) as i8 as i32;

        bridge.add_child(dev, fun, new_bridge);
    }

    assert_eq!(
        AcpiGetDevices(null_mut(), Some(add_pci_dev), Object, null_mut()),
        AE_OK
    );

    return AE_OK;
}

#[derive(Copy, Clone)]
struct AcpiHandle(ACPI_HANDLE);

unsafe impl Sync for AcpiHandle {}
unsafe impl Send for AcpiHandle {}

static ROOT_BRIDGE: Once<Spin<PciBridge>> = Once::new();
static ROOT_HANDLE: Once<AcpiHandle> = Once::new();

fn root_bridge<'a>() -> SpinGuard<'a, PciBridge> {
    ROOT_BRIDGE.r#try().unwrap().lock()
}

fn root_handle() -> ACPI_HANDLE {
    ROOT_HANDLE.r#try().unwrap().0
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
unsafe extern "C" fn pci_add_root_dev(
    Object: ACPI_HANDLE,
    _NestingLevel: UINT32,
    _Context: *mut ::core::ffi::c_void,
    _ReturnValue: *mut *mut ::core::ffi::c_void,
) -> ACPI_STATUS {
    println!("[ ACPI ] Found PCI root bridge");

    let mut pci = PciBridge::new(Object);
    pci.primary = -1;
    pci.secondary = 0;

    ROOT_HANDLE.call_once(|| AcpiHandle(pci.acpi_handle));
    ROOT_BRIDGE.call_once(|| Spin::new(pci));

    root_bridge().init();

    AE_OK
}

struct PciBridge {
    acpi_handle: ACPI_HANDLE,
    irqs: [i32; 32 * 4],

    device: i32,
    function: i32,

    primary: i32,
    secondary: i32,

    children: BTreeMap<(i32, i32), Arc<PciBridge>>,
}

unsafe impl Sync for PciBridge {}
unsafe impl Send for PciBridge {}

struct ResData {
    bridge: *mut PciBridge,
    tbl: *mut acpi_pci_routing_table,
}

impl PciBridge {
    fn new(handle: ACPI_HANDLE) -> PciBridge {
        PciBridge {
            acpi_handle: handle,
            irqs: [-1; 32 * 4],
            device: -1,
            function: -1,
            primary: -1,
            secondary: -1,
            children: BTreeMap::new(),
        }
    }

    fn add_irq(&mut self, dev: u64, pin: u8, int: u32) {
        println!("[ ACPI ] Add irq {} {} {}", dev, pin, int);
        self.irqs[dev as usize * 4 + pin as usize] = int as i32;
    }

    fn init_dev_fun(&mut self) -> (i32, i32) {
        let mut dev_info: *mut ACPI_DEVICE_INFO = null_mut();

        unsafe {
            assert_eq!(
                AcpiGetObjectInfo(
                    self.acpi_handle,
                    &mut dev_info as *mut *mut ACPI_DEVICE_INFO
                ),
                AE_OK
            );
        }

        let dev = unsafe { (*dev_info).Address as u32 >> 16 } as i32;

        let fun = unsafe { (*dev_info).Address & 0xFFFF } as i32;

        self.device = dev as i32;
        self.function = fun as i32;

        (dev, fun)
    }

    fn init_irq_routing(&mut self) -> bool {
        let mut buf: ACPI_BUFFER = ACPI_BUFFER {
            Length: -1isize as ACPI_SIZE, // ACPI_ALLOCATE_BUFFER
            Pointer: null_mut(),
        };

        let status =
            unsafe { AcpiGetIrqRoutingTable(self.acpi_handle, &mut buf as *mut ACPI_BUFFER) };

        if status != AE_OK {
            return false;
        }

        let mut tbl = unsafe { &mut *(buf.Pointer as *mut acpi_pci_routing_table) };

        while tbl.Length != 0 {
            if tbl.Source[0] == 0 {
                self.add_irq(tbl.Address >> 16, tbl.Pin as u8, tbl.SourceIndex);
            } else {
                let mut src_handle: ACPI_HANDLE = null_mut();

                unsafe {
                    assert_eq!(
                        AcpiGetHandle(
                            self.acpi_handle,
                            tbl.Source.as_mut_ptr(),
                            &mut src_handle as *mut ACPI_HANDLE
                        ),
                        AE_OK
                    );
                }

                let mut data = ResData {
                    bridge: self as *mut PciBridge,
                    tbl: tbl as *mut acpi_pci_routing_table,
                };

                unsafe {
                    assert_eq!(
                        AcpiWalkResources(
                            src_handle,
                            acpi_str(b"_CRS\0"),
                            Some(get_irq_resource),
                            &mut data as *mut ResData as *mut core::ffi::c_void
                        ),
                        AE_OK
                    );
                }
            }

            unsafe {
                buf.Pointer = buf.Pointer.offset(tbl.Length as isize);
                tbl = &mut *(buf.Pointer as *mut acpi_pci_routing_table);
            }
        }

        return true;
    }

    fn init(&mut self) {
        self.init_dev_fun();

        self.init_irq_routing();

        unsafe {
            assert_eq!(
                AcpiGetDevices(
                    null_mut(),
                    Some(add_pci_dev),
                    self as *mut PciBridge as *mut core::ffi::c_void,
                    null_mut()
                ),
                AE_OK
            );
        }
    }

    fn add_child(&mut self, dev: i32, fun: i32, bridge: PciBridge) {
        println!(
            "[ ACPI ] Adding bridge {} -> {}",
            bridge.primary, bridge.secondary
        );
        self.children.insert((dev, fun), Arc::new(bridge));
    }
}

pub fn pci_routing() {
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

pub fn init() {
    call_pic1();

    pci_routing();
}
